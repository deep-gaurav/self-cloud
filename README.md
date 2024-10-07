<h2 align="center"><b>SelfCloud</b></h2>
<h4 align="center">Your Personal, Self-Hosted PaaS Solution</h4>

<hr>

## Screenshots

[<img src="https://deepgaurav.com/assets/images/projects/selfcloud/sc1-2880.avif" height=300>](SC1)
[<img src="https://deepgaurav.com/assets/images/projects/selfcloud/sc2-2880.avif" height=300>](SC2)
[<img src="https://deepgaurav.com/assets/images/projects/selfcloud/sc3-2880.avif" height=300>](SC3)
[<img src="https://deepgaurav.com/assets/images/projects/selfcloud/sc4-2880.avif" height=300>](SC4)
[<img src="https://deepgaurav.com/assets/images/projects/selfcloud/sc5-2880.avif" height=300>](SC5)
[<img src="https://deepgaurav.com/assets/images/projects/selfcloud/sc6-2880.avif" height=300>](SC6)


## Overview

SelfCloud empowers developers to create their own Platform as a Service (PaaS) environment. Built entirely in Rust using the Leptos framework, it provides a robust, efficient, and user-friendly solution for deploying and managing containerized applications.

## Features

- **Project Management**: Create and manage multiple projects within your SelfCloud instance.
- **Custom Domain Support**: Assign custom domains to your projects with ease.
- **Automated SSL Management**: SelfCloud handles SSL certificate generation and renewal automatically.
- **Container Deployment**: Push and deploy Docker containers to your projects using generated tokens.
- **Port Exposure**: Automatically exposes configured ports to assigned domains.
- **Support Containers**: Deploy additional containers like databases to support your main application.
- **Monitoring and Logs**: Keep track of your application's performance and logs in real-time.
- **Terminal Access**: Attach a terminal to your containers for direct interaction and debugging.

## How It Works

1. Create a new project in SelfCloud and assign it a domain.
2. Generate an authentication token for the project.
3. Use the token to push your Docker container to SelfCloud.
4. SelfCloud deploys your container and exposes the configured port to the assigned domain.
5. Monitor your application's performance, view logs, and access the terminal as needed.

## Technical Details

- **Backend & Frontend**: Built with Rust using the Leptos framework for a unified, full-stack development experience.
- **Containerization**: Utilizes Docker for application deployment and isolation.
- **SSL Management**: Integrates with Let's Encrypt for automated SSL certificate provisioning and renewal.
- **Reverse Proxy**: Employs a reverse proxy to route traffic to the appropriate containers based on domain configuration.

## Key Advantages

- **Self-Hosted**: Maintain full control over your deployment environment and data.
- **Cost-Effective**: Eliminate ongoing PaaS subscription costs by hosting on your own infrastructure.
- **Customizable**: Tailor the platform to your specific needs and workflows.
- **Secure**: Benefit from Rust's security features and automated SSL management.
- **Performance**: Leverage Rust's speed and efficiency for optimal resource utilization.

## Use Cases

- **Personal Projects**: Deploy and manage your side projects with ease.
- **Small Teams**: Provide a centralized deployment solution for team projects.
- **Education**: Set up a controlled environment for teaching deployment and DevOps concepts.
- **Prototyping**: Quickly deploy and iterate on new ideas without complex setup.

Experience the power and flexibility of your own personal PaaS with SelfCloud - where control meets convenience in application deployment!

## License
[![GNU GPLv3 Image](https://www.gnu.org/graphics/gplv3-127x51.png)](https://www.gnu.org/licenses/gpl-3.0.html)  

SelfCloud is Free Software: You can use, study share and improve it at your
will. Specifically you can redistribute and/or modify it under the terms of the
[GNU General Public License](https://www.gnu.org/licenses/gpl-3.0.html) as
published by the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
