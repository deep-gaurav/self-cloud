use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use app::common::{SSLProvisioning, DOMAIN_MAPPING};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, KeyAuthorization,
    LetsEncrypt, NewAccount, NewOrder, OrderStatus,
};
use rcgen::{
    Certificate, CertificateParams, CertificateSigningRequest, DistinguishedName, KeyPair,
};
use tracing::info;

use crate::leptos_service::AppState;

pub type TLSState = Arc<RwLock<HashMap<String, KeyAuthorization>>>;

pub async fn acme_handler(
    State(app_state): State<AppState>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    let tls = app_state.tls_state.read().unwrap();
    if let Some(key) = tls.get(&token) {
        (StatusCode::OK, format!("{}", key.as_str()))
    } else {
        (StatusCode::NOT_FOUND, format!("Not Found"))
    }
}

pub async fn tls_generator(acme: TLSState) -> anyhow::Result<()> {
    let mut account = if let Ok(bytes) = tokio::fs::read("account.json").await {
        'ba: {
            if let Ok(cred) = serde_json::from_slice::<AccountCredentials>(&bytes) {
                let account = Account::from_credentials(cred)
                    .await
                    .map_err(|e| anyhow::anyhow!("cant restore lets encrypt account {e:#?}"))?;
                info!("Using existing account");
                break 'ba Some(account);
            }
            None
        }
    } else {
        None
    };

    if account.is_none() {
        info!("Creating new account");
        let (account_new, credentials) = Account::create(
            &NewAccount {
                contact: &[],
                terms_of_service_agreed: true,
                only_return_existing: false,
            },
            LetsEncrypt::Staging.url(),
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("cant create lets encrypt account {e:#?}"))?;

        let data = serde_json::to_vec(&credentials)
            .map_err(|e| anyhow::anyhow!("Cannot serialize {e:#?}"))?;
        tokio::fs::write("account.json", data)
            .await
            .map_err(|e| anyhow::anyhow!("Cannot write account {e:#?}"))?;

        account = Some(account_new);
    }

    let account = account.unwrap();

    loop {
        let domain = 'ba: {
            let mut peers = DOMAIN_MAPPING.write().unwrap();
            for (domain, peer) in peers.iter_mut() {
                if peer.ssl_provision.is_not_provisioned() {
                    peer.ssl_provision = SSLProvisioning::Provisioning;
                    break 'ba Some(domain.clone());
                }
            }
            None
        };
        let account = account.clone();
        let acme = acme.clone();

        tokio::spawn(async move {
            if let Some(domain) = domain {
                let identifier = Identifier::Dns(domain.clone());
                let mut order = account
                    .new_order(&NewOrder {
                        identifiers: &[identifier],
                    })
                    .await
                    .unwrap();

                let state = order.state();
                info!("order state: {:#?}", state);

                // Pick the desired challenge type and prepare the response.

                if state.status == OrderStatus::Pending {
                    let authorizations = order.authorizations().await.unwrap();
                    let mut challenges = Vec::with_capacity(authorizations.len());
                    for authz in &authorizations {
                        match authz.status {
                            AuthorizationStatus::Pending => {}
                            AuthorizationStatus::Valid => continue,
                            _ => todo!(),
                        }
                        let challenge = authz
                            .challenges
                            .iter()
                            .find(|c| c.r#type == ChallengeType::Http01)
                            .ok_or_else(|| anyhow::anyhow!("no http01 challenge found"));
                        let challenge = match challenge {
                            Ok(c) => c,
                            Err(e) => {
                                tracing::error!("{e:#?}");
                                return;
                            }
                        };

                        info!("Found challenge {challenge:#?}");

                        let Identifier::Dns(identifier) = &authz.identifier;

                        {
                            let mut acme = acme.write().unwrap();
                            acme.insert(
                                challenge.token.clone(),
                                order.key_authorization(challenge),
                            );
                        }

                        challenges.push((identifier, &challenge.url));
                    }

                    // Let the server know we're ready to accept the challenges.

                    for (_, url) in &challenges {
                        order.set_challenge_ready(url).await.unwrap();
                    }
                }

                // Exponentially back off until the order becomes ready or invalid.

                let mut tries = 1u8;
                let mut delay = std::time::Duration::from_millis(250);
                loop {
                    tokio::time::sleep(delay).await;
                    let state = order.refresh().await.unwrap();
                    if let OrderStatus::Ready | OrderStatus::Invalid = state.status {
                        info!("order state: {:#?}", state);
                        break;
                    }

                    delay *= 2;
                    tries += 1;
                    match tries < 20 {
                        true => info!(?state, tries, "order is not ready, waiting {delay:?}"),
                        false => {
                            tracing::error!(tries, "order is not ready: {state:#?}");
                            return;
                        }
                    }
                }

                let state = order.state();
                if state.status != OrderStatus::Ready {
                    tracing::error!("unexpected order status: {:?}", state.status);
                    return;
                }

                let mut names = vec![domain.clone()];

                // If the order is ready, we can provision the certificate.
                // Use the rcgen library to create a Certificate Signing Request.

                let mut params = CertificateParams::new(names.clone()).unwrap();
                params.distinguished_name = DistinguishedName::new();
                let kp = KeyPair::generate().unwrap();
                let cert = params.serialize_request(&kp).unwrap();
                let csr = cert.der();

                // Finalize the order and print certificate chain, private key and account credentials.

                order.finalize(&csr).await.unwrap();
                let cert_chain_pem = loop {
                    match order.certificate().await.unwrap() {
                        Some(cert_chain_pem) => break cert_chain_pem,
                        None => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
                    }
                };

                // info!("certficate chain:\n\n{}", cert_chain_pem);
                // info!("private key:\n\n{}", kp.serialize_pem());

                tokio::fs::create_dir_all(format!("certificates/{}", domain))
                    .await
                    .unwrap();
                tokio::fs::write(
                    format!("certificates/{}/cert.pem", domain),
                    cert_chain_pem.clone(),
                )
                .await
                .expect("cant write cert");
                tokio::fs::write(
                    format!("certificates/{}/key.pem", domain),
                    kp.serialize_pem(),
                )
                .await
                .expect("cant write key");

                {
                    let mut peers = DOMAIN_MAPPING.write().unwrap();
                    if let Some(peer) = peers.get_mut(&domain) {
                        let cert = pingora::tls::x509::X509::from_pem(cert_chain_pem.as_bytes());
                        let key = pingora::tls::pkey::PKey::private_key_from_pem(
                            kp.serialize_pem().as_bytes(),
                        );

                        if let (Ok(cert), Ok(key)) = (cert, key) {
                            peer.ssl_provision =
                                SSLProvisioning::Provisioned(app::common::SSlData { cert, key });
                        }
                    }
                };
            }
        });
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
