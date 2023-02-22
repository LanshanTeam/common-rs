use crate::config::env::optional;
use crate::infra::Resolver;
use crate::middleware::apollo::{Apollo, ApolloConf};
use crate::middleware::Middleware;
use colored::Colorize;
use kosei::{Config, ConfigType};
use serde::Serialize;
use std::cmp::Ordering;
use std::path::Path;
use crate::middleware::nacos::{Nacos, NacosConf};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn parse_config<R: Resolver>() -> Result<R::Config, Error> {
    let typ = optional("CONFIG_TYPE", "file");
    match typ.to_lowercase().as_str() {
        "file" => {
            let path = optional("CONFIG_PATH", "config");
            let path: &Path = path.as_ref();

            // parse config from directory with service_domain
            if path.is_dir() {
                let path = path.join(format!(
                    "{}.{}.{}",
                    R::DOMAIN,
                    R::TARGET,
                    optional("CONFIG_FILETYPE", "yml")
                ));
                if path.exists() {
                    return Ok(Config::<R::Config>::from_file(path).into_inner());
                }
            }
            if path.exists() {
                return Ok(Config::<R::Config>::from_file(path).into_inner());
            }
            Ok(Config::<R::Config>::new("".to_string(), ConfigType::YAML).into_inner())
        }
        "apollo" => {
            let apollo = Apollo::new(ApolloConf::default());
            let client = apollo.make_client().await.unwrap();

            Ok(Config::<R::Config>::from_apollo(&client)
                .await?
                .into_inner())
        }
        "nacos" => {
            let nacos = Nacos::new(NacosConf::default());
            let mut client = nacos.make_client().await.unwrap();

            Ok(Config::<R::Config>::from_nacos(&mut client)
                .await?
                .into_inner())
        }
        _ => panic!("unsupported config type"),
    }
}

pub fn config_tips<T: Serialize>(config: &T) {
    let tips = "That is your configuration";
    let words = serde_json::to_string_pretty(&config).unwrap();
    let mut format_lines = vec!["╭".to_string()];
    for line in words.lines() {
        format_lines.push(format!("│ {}", line))
    }
    let mut width = format_lines
        .iter()
        .max_by(|lhs, rhs| {
            if lhs.len() > rhs.len() {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        })
        .map(|v| v.len())
        .unwrap()
        .max(tips.len() + 3);
    format_lines.iter_mut().for_each(|line| {
        while line.chars().count() <= width {
            if line.starts_with('╭') {
                line.push('─');
            } else if line.starts_with('│') {
                line.push(' ');
            }
        }
        if line.starts_with('╭') {
            line.push('╮');
        } else if line.starts_with('│') {
            line.push('│');
        }
    });
    width -= tips.len() + 2;
    format_lines.push(format!(
        "╰{} {} {}╯",
        "─".repeat(width / 2),
        tips,
        "─".repeat(width / 2 + width % 2)
    ));
    println!("\n{}\n", format_lines.join("\n").bright_green());
}

pub mod regex {
    use once_cell::sync::OnceCell;
    use regex::Regex;

    static EMAIL_REGEX: OnceCell<Regex> = OnceCell::new();
    static CN_PHONE_REGEX: OnceCell<Regex> = OnceCell::new();
    static US_PHONE_REGEX: OnceCell<Regex> = OnceCell::new();

    #[inline]
    pub fn check_email(str: &str) -> bool {
        EMAIL_REGEX
            .get_or_init(|| {
                Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
            })
            .is_match(str)
    }

    #[cfg(test)]
    #[test]
    fn test_email() {
        assert!(check_email("114514@qq.com"));
        assert!(check_email("igxnon@gmail.com"));
        assert!(!check_email("igxnon:gmail.com"));
        assert!(!check_email("igxnon@gmailcom"));
    }

    #[inline]
    pub fn check_cn_phone(str: &str) -> bool {
        CN_PHONE_REGEX
            .get_or_init(|| Regex::new(r"^1[3-9]\d{9}$").unwrap())
            .is_match(str)
    }

    #[cfg(test)]
    #[test]
    fn test_cn_phone() {
        assert!(check_cn_phone("13847722940"));
        assert!(check_cn_phone("13039947289"));
        assert!(!check_cn_phone("igxnon:gmail.com"));
        assert!(!check_cn_phone("igxnon@gmailcom"));
    }

    pub fn check_us_phone(str: &str) -> bool {
        US_PHONE_REGEX
            .get_or_init(|| Regex::new(r"^\d{3}-\d{3}-\d{4}$").unwrap())
            .is_match(str)
    }

    #[cfg(test)]
    #[test]
    fn test_us_phone() {
        assert!(check_us_phone("123-456-7890"));
        assert!(check_us_phone("224-456-7890"));
        assert!(!check_us_phone("igxnon:gmail.com"));
        assert!(!check_us_phone("igxnon@gmailcom"));
    }
}
