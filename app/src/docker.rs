use std::collections::HashMap;

use bollard::{container::{Config, CreateContainerOptions}, secret::{HostConfig, Mount, MountTypeEnum, PortBinding}, Docker};

use crate::{config::CLI_CONFIG, error::CliResult};

/// Start a postgres container on the machine
pub async fn start_pg_container() -> CliResult<()> {
    let docker = Docker::connect_with_local_defaults()?;

    let env = vec![
        format!("POSTGRES_USER={}", CLI_CONFIG.local_postgres.user),
        format!("POSTGRES_PASSWORD={}", CLI_CONFIG.local_postgres.pass),
        format!("POSTGRES_DB={}", CLI_CONFIG.local_postgres.database),
    ];

    let mut volumes = HashMap::new();
    volumes.insert(
        String::from("/var/lib/postgresql/data"),
        HashMap::new(),
    );

    let host_config = HostConfig {
        port_bindings: Some({
            let mut ports = HashMap::new();
            ports.insert(
                String::from("5432/tcp"),
                Some(vec![PortBinding {
                    host_ip: Some(CLI_CONFIG.local_postgres.host.clone()),
                    host_port: Some(CLI_CONFIG.local_postgres.port.clone()),
                }]),
            );
            ports
        }),
        mounts: Some(
            vec![
                Mount {
                    target: Some(String::from("/var/lib/postgresql/data")),
                    source: Some(String::from("ocloud-data")),
                    typ: Some(MountTypeEnum::VOLUME),
                    ..Default::default()
                }
            ]
        ),
        ..Default::default()
    };

    let container_config = Config {
        image: Some(String::from("postgres:16")),
        env: Some(env),
        host_config: Some(host_config),
        volumes: Some(volumes),
        ..Default::default()
    };

    docker
        .create_container(
            Some(CreateContainerOptions {
                name: &CLI_CONFIG.local_postgres.container_name,
                platform: None
            }),
            container_config,
        )
        .await?;

    docker
        .create_volume(bollard::volume::CreateVolumeOptions {
            name: "ocloud-data",
            ..Default::default()
        })
        .await?;

    docker
        .start_container::<String>(&CLI_CONFIG.local_postgres.container_name, None)
        .await?;

    println!("Database container started successfully");

    Ok(())
}