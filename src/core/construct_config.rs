use crate::core::cli::{ConfigArgs, ConfigFileProvider};
use crate::domain::config::{
    ConfigBuilder, DomainConfigBuilder, TlsConfigBuilder, DEFAULT_SITE_FOLDER,
};
use std::io::stdin;
use tokio::fs;
use url::Url;

pub async fn construct_config(args: &ConfigArgs) {
    if !do_not_mind_overwriting(args).await {
        return;
    }

    let mut config_builder = &mut ConfigBuilder::default();

    config_builder.exit_after(None);
    config_builder = read_email(config_builder);
    config_builder = read_domain_config(config_builder);
    config_builder = read_site_state_folder(config_builder);

    let mut config = config_builder
        .build()
        .unwrap_or_else(|error| panic!("Could not build the config because {:?}", error));

    if config.uses_tls()
        && config.domain_config().builder_contacts().is_empty()
        && config.email().is_some()
    {
        let mut domain_config = config.domain_config().clone();
        let tls_config = config
            .domain_config()
            .tls_config()
            .as_ref()
            .unwrap()
            .clone();
        domain_config = domain_config.with_tls_config(
            tls_config.with_builder_contacts(vec![config.email().as_ref().unwrap().to_string()]),
        );
        config = config.with_domain_config(domain_config);
    }

    config.sanity_check().expect("Config is not valid");

    fs::write(
        args.config_file(),
        serde_yaml::to_string(&config).expect("Error serializing config"),
    )
    .await
    .expect("Error writing config file");
}

fn read_domain_config(config_builder: &mut ConfigBuilder) -> &mut ConfigBuilder {
    let domain_builder = &mut DomainConfigBuilder::default();
    let url = read_optional_string("Please enter the url you'd like to use for serving web traffic (leave blank for http://localhost):");

    if let Some(url) = url {
        let url = Url::parse(&url).expect("Error parsing URL");

        let external_url = read_optional_string(
            "If you would like to use a different URL visible externally, please provide it here (leave blank if the URL you entered above is visible externally):",
        )
        .map(|url| Url::parse(&url).expect("Error parsing external URL"));

        if url.scheme() == "https" || external_url.as_ref().map(|url| url.scheme()) == Some("https")
        {
            let mut tls_config = &mut TlsConfigBuilder::default();
            tls_config = read_builder_contacts(tls_config);

            let mut buf = String::new();
            println!(
                "Please enter the number of poll attempts to make when retrieving a domain certificate:"
            );
            buf.clear();
            stdin()
                .read_line(&mut buf)
                .expect("Error reading poll attempts");
            tls_config.poll_attempts(
                buf.trim()
                    .parse::<usize>()
                    .expect("Error parsing poll attempts"),
            );
            println!("Please enter the number of seconds to wait between poll attempts:");
            buf.clear();
            stdin()
                .read_line(&mut buf)
                .expect("Error reading poll interval seconds");
            tls_config.poll_interval_seconds(
                buf.trim()
                    .parse::<u64>()
                    .expect("Error parsing poll interval seconds"),
            );
            tls_config.cert_location(read_optional_string(
                "Please enter the folder where the site cert should be stored (leave blank for site state folder):",
            ));
            domain_builder.tls_config(Some(tls_config.build().expect("Error building TLS config")));
        } else {
            domain_builder.tls_config(None);
        }

        domain_builder.url(url);
        domain_builder.external_url(external_url);

        config_builder.domain_config(Some(
            domain_builder
                .build()
                .expect("Error building domain config"),
        ));
    } else {
        config_builder.domain_config(Default::default());
    }

    config_builder
}

fn read_builder_contacts(tls_config: &mut TlsConfigBuilder) -> &mut TlsConfigBuilder {
    let mut builder_contacts: Vec<String> = vec![];
    loop {
        println!("Please enter the email you'd like to use as a contact for the domain (leave blank to finish, an empty list will be replaced with the admin contact email):");
        let mut buf = String::new();
        stdin().read_line(&mut buf).expect("Error reading email");
        let email = buf.trim();
        if email.is_empty() {
            break;
        }
        builder_contacts.push(email.to_string());
    }

    tls_config.builder_contacts(builder_contacts);
    tls_config
}

fn read_email(config_builder: &mut ConfigBuilder) -> &mut ConfigBuilder {
    config_builder.email(read_optional_string("Please enter the email you'd like to use as the admin contact when requesting a certificate (you can leave this blank if you don't want to host a secure site using HTTPS):"))
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
}

fn read_boolean(message: &str, default_yes: bool) -> bool {
    let default_message = if default_yes { "Y/n" } else { "y/N" };
    let result;
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

fn read_optional_string(message: &str) -> Option<String> {
    println!("{}", message);

    let mut input = String::new();
    stdin().read_line(&mut input).expect("Error reading input");

    let result = input.trim().to_lowercase();

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}
