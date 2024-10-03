use crate::core::cli::{ConfigArgs, ConfigFileProvider};
use crate::domain::config::{ConfigBuilder, DomainConfigBuilder, DEFAULT_SITE_FOLDER, TLS_PORT};
use std::io::stdin;
use tokio::fs;

pub async fn construct_config(args: &ConfigArgs) {
    if !do_not_mind_overwriting(args).await {
        return;
    }

    let mut config_builder = &mut ConfigBuilder::default();

    config_builder.exit_after(None);
    config_builder = read_domain_config(config_builder).await;
    config_builder = read_email(config_builder);
    config_builder = read_tls_port(config_builder);
    config_builder = read_site_state_folder(config_builder);

    let config = config_builder
        .build()
        .unwrap_or_else(|error| panic!("Could not build the config because {:?}", error));
    config.sanity_check().expect("Config is not valid");

    fs::write(
        args.config_file(),
        serde_yaml::to_string(&config).expect("Error serializing config"),
    )
    .await
    .expect("Error writing config file");
}

async fn read_domain_config(config_builder: &mut ConfigBuilder) -> &mut ConfigBuilder {
    if read_boolean("Would you like to configure a domain?", true).await {
        let mut domain_builder = &mut DomainConfigBuilder::default();
        domain_builder = read_builder_contacts(domain_builder);
        println!("Please enter the domain you'd like to serve:");
        let mut buf = String::new();
        stdin().read_line(&mut buf).expect("Error reading domain");
        domain_builder.domain_name(buf.trim().to_string());
        println!("Please enter the number of poll attempts to make when retrieving a domain certificate:");
        buf.clear();
        stdin()
            .read_line(&mut buf)
            .expect("Error reading poll attempts");
        domain_builder.poll_attempts(
            buf.trim()
                .parse::<usize>()
                .expect("Error parsing poll attempts"),
        );
        println!("Please enter the number of seconds to wait between poll attempts:");
        buf.clear();
        stdin()
            .read_line(&mut buf)
            .expect("Error reading poll interval seconds");
        domain_builder.poll_interval_seconds(
            buf.trim()
                .parse::<u64>()
                .expect("Error parsing poll interval seconds"),
        );
        config_builder.domain_config(Some(
            domain_builder
                .build()
                .expect("Error building domain config"),
        ));
    } else {
        config_builder.domain_config(None);
    }

    config_builder
}

fn read_builder_contacts(
    domain_config_builder: &mut DomainConfigBuilder,
) -> &mut DomainConfigBuilder {
    let mut builder_contacts: Vec<String> = vec![];
    loop {
        println!("Please enter the email you'd like to use as a contact for the domain (leave blank to finish):");
        let mut buf = String::new();
        stdin().read_line(&mut buf).expect("Error reading email");
        let email = buf.trim();
        if email.is_empty() {
            break;
        }
        builder_contacts.push(email.to_string());
    }

    domain_config_builder.builder_contacts(builder_contacts);
    domain_config_builder
}

fn read_email(config_builder: &mut ConfigBuilder) -> &mut ConfigBuilder {
    println!("Please enter the email you'd like to use as the admin contact when requesting a certificate:");
    let mut buf = String::new();
    stdin().read_line(&mut buf).expect("Error reading email");
    let email = buf.trim();
    if email.is_empty() {
        config_builder.email(None);
    } else {
        config_builder.email(Some(email.to_string()));
    }

    config_builder
}

fn read_tls_port(config_builder: &mut ConfigBuilder) -> &mut ConfigBuilder {
    let mut buf = String::new();
    println!(
        "Please enter the port you'd like to use for serving HTTPS traffic (leave blank for {}):",
        TLS_PORT
    );
    stdin().read_line(&mut buf).expect("Error reading port");

    if buf.trim().is_empty() {
        config_builder.port(None);
        return config_builder;
    }

    let port = buf.trim().parse::<u16>().expect("Error parsing port");
    config_builder.port(Some(port));
    config_builder
}

fn read_site_state_folder(config_builder: &mut ConfigBuilder) -> &mut ConfigBuilder {
    let mut buf = String::new();
    println!(
        "Please enter the folder where site state will be stored (leave blank for {}):",
        DEFAULT_SITE_FOLDER
    );
    stdin()
        .read_line(&mut buf)
        .expect("Error reading site state folder");

    if buf.trim().is_empty() {
        config_builder.site_state_folder(None);
        return config_builder;
    }

    config_builder.site_state_folder(Some(buf.trim().to_string()));
    config_builder
}

async fn do_not_mind_overwriting(config_args: &ConfigArgs) -> bool {
    if !fs::try_exists(config_args.config_file())
        .await
        .expect("Error checking config file existence")
    {
        return true;
    }

    read_boolean(
        &format!(
            "Config file {} already exists, overwrite?",
            config_args.config_file()
        ),
        false,
    )
    .await
}

async fn read_boolean(message: &str, default_yes: bool) -> bool {
    let default_message = if default_yes { "Y/n" } else { "y/N" };
    let mut result;
    loop {
        println!("{} ({})", message, default_message);
        let mut input = String::new();
        if stdin().read_line(&mut input).is_err() {
            println!("Error reading input, please try again");
            continue;
        }
        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" {
            result = true;
            break;
        } else if input == "n" || input == "no" {
            result = false;
            break;
        } else if input == "" {
            result = default_yes;
            break;
        } else {
            println!("Invalid input, please enter 'y' or 'n'");
            continue;
        }
    }
    result
}
