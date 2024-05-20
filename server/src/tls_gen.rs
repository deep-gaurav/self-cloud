use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt,
    NewAccount, NewOrder, OrderStatus,
};
use tracing::info;

use crate::PEERS;

async fn tls_generator() -> anyhow::Result<()> {
    let mut account = if let Ok(bytes) = tokio::fs::read("account.json").await {
        'ba: {
            if let Ok(cred) = serde_json::from_slice::<AccountCredentials>(&bytes) {
                let account = Account::from_credentials(cred)
                    .await
                    .map_err(|e| anyhow::anyhow!("cant restore lets encrypt account {e:#?}"))?;
                break 'ba Some(account);
            }
            None
        }
    } else {
        None
    };

    if account.is_none() {
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
            let peers = PEERS.read().unwrap();
            for (domain, peer) in peers.iter() {
                if peer.provisioning.is_not_provisioned() {
                    break 'ba Some(domain.clone());
                }
            }
            None
        };
        if let Some(domain) = domain {
            let identifier = Identifier::Dns(domain);
            let mut order = account
                .new_order(&NewOrder {
                    identifiers: &[identifier],
                })
                .await
                .unwrap();

            let state = order.state();
            info!("order state: {:#?}", state);
            assert!(matches!(state.status, OrderStatus::Pending));

            // Pick the desired challenge type and prepare the response.

            let authorizations = order.authorizations().await.unwrap();
            let mut challenges = Vec::with_capacity(authorizations.len());
            for authz in &authorizations {
                match authz.status {
                    AuthorizationStatus::Pending => {}
                    AuthorizationStatus::Valid => continue,
                    _ => todo!(),
                }

                // We'll use the DNS challenges for this example, but you could
                // pick something else to use here.

                let challenge = authz
                    .challenges
                    .iter()
                    .find(|c| c.r#type == ChallengeType::Http01)
                    .ok_or_else(|| anyhow::anyhow!("no http01 challenge found"))?;

                info!("Found challenge {challenge:#?}");

                let Identifier::Dns(identifier) = &authz.identifier;

                println!("Please set the following DNS record then press the Return key:");
                println!(
                    "_acme-challenge.{} IN TXT {}",
                    identifier,
                    order.key_authorization(challenge).as_str()
                );

                challenges.push((identifier, &challenge.url));
            }

            // Let the server know we're ready to accept the challenges.
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
