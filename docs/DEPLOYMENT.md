# SelfCloud Capabilities & Integration Guide

This document outlines the features, architectural constraints, and deployment capabilities of SelfCloud. Use this guide to structure your projects and CI/CD pipelines effectively.

## 1. Core Concepts

SelfCloud organizes workloads into **Projects**. A project is an isolated unit that can be one of two types:
* **Container Project**: A full Docker-based deployment with a primary application and optional support services.
* **Port Forward**: A simple tunnel to an existing local port (useful for internal tools or testing).

---

## 2. Container Project Capabilities

The **Container Project** is the primary deployment model. It supports the following features:

### A. Primary Application
* **Single Primary Container**: Each project has one "Primary Container" which is the main application receiving updates via the image uploader.
* **Automatic Rolling Updates**: Pushing a new image automatically stops the old container and starts the new one with the same configuration.
* **Environment Variables**: Supports defining a list of key-value pairs (`env_vars`) injected into the container at runtime.

### B. Networking & Multiple Ports
SelfCloud allows you to expose multiple ports from your primary container and route them to different domains.

* **Multi-Port Exposure**: You can define multiple `ExposedPort` entries.
    * *Example:* You can expose port `80` for the web UI and port `3000` for an API from the same container.
* **Domain Routing**: Each exposed port can be mapped to specific domains.
    * *Port 80* -> `myapp.example.com`
    * *Port 3000* -> `api.myapp.example.com`
    * *Traffic Routing:* SelfCloud handles the reverse proxying (using Pingora) to route incoming traffic based on the domain to the correct internal container port.

### C. Support Containers (Sidecars)
You can define auxiliary containers (e.g., databases, Redis, caches) alongside your primary application.

* **Internal Access Only**: Support containers are **not** exposed to the public internet. They are strictly for internal use by the primary container.
* **Service Discovery**:
    * All containers in a project share a dedicated Docker network (`selfcloud_network_{project_id}`).
    * The primary container can reach support containers using their **name** as the hostname.
    * *Example:* If you name a support container `postgres`, your app connects via `postgres:5432`.
* **Lifecycle**: Support containers are managed by SelfCloud and started alongside the project.

### D. SSL/TLS Management
* **Automatic Provisioning**: Domains attached to projects support automatic SSL provisioning (likely Let's Encrypt/ACME based on `SSLProvisioning` states).

---

## 3. Deployment & CI/CD Integration

SelfCloud supports direct image uploads via HTTP, making it easy to integrate into GitHub Actions or other CI pipelines.

### Push Endpoint
* **URL**: `/cloud/image/push`
* **Method**: `POST` (Multipart Form Data)
* **Authentication**: Requires a specific `token` validated against the project configuration.

### Example CI Workflow
To deploy a new version of your primary container:

```bash
curl --location --fail --show-error \
  '[https://your-selfcloud-instance.com/cloud/image/push](https://your-selfcloud-instance.com/cloud/image/push)' \
  --form 'project_id="<YOUR_PROJECT_UUID>"' \
  --form "token=$SELF_CLOUD_TOKEN" \
  --form 'image=@"release.tar.gz"'
