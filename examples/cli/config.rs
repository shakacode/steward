use std::{collections::HashMap, iter::FromIterator, sync::LazyLock};

use crate::loc;

#[derive(Clone)]
pub struct Config {
    data: HashMap<String, String>,
}

// We want to load .env file once per execution and then reuse it
static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

impl Config {
    fn load() -> Self {
        #[allow(deprecated)] // it was undeprecated
        let data = HashMap::from_iter(
            dotenv::from_path_iter(loc::env_file().path())
                .unwrap()
                .map(Result::unwrap),
        );
        Self { data }
    }

    pub fn to_env() -> steward::Env {
        steward::Env::new(CONFIG.data.to_owned())
    }
}

impl Config {
    #[allow(non_snake_case)]
    pub fn SERVER_HOST() -> &'static str {
        CONFIG
            .data
            .get("SERVER_HOST")
            .expect("SERVER_HOST is not set")
    }

    #[allow(non_snake_case)]
    pub fn SERVER_PORT() -> &'static str {
        CONFIG
            .data
            .get("SERVER_PORT")
            .expect("SERVER_PORT is not set")
    }
}
