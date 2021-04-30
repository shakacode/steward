use std::{collections::HashMap, iter::FromIterator};

use crate::Loc;

#[derive(Clone)]
pub struct Config {
    data: HashMap<String, String>,
}

lazy_static! {
    // We want to load .env file once per execution and then reuse it
    static ref CONFIG: Config = Config::load();
}

impl Config {
    fn load() -> Self {
        #[allow(deprecated)] // it was undeprecated
        let data = HashMap::from_iter(
            dotenv::from_path_iter(Loc::env_file().path())
                .unwrap()
                .map(Result::unwrap),
        );
        Self { data }
    }

    pub fn to_env() -> steward::Env {
        steward::Env::new(CONFIG.data.to_owned())
    }
}

impl<'a> Config {
    #[allow(non_snake_case, dead_code)]
    pub fn SERVER_HOST() -> &'a str {
        CONFIG
            .data
            .get("SERVER_HOST")
            .expect("SERVER_HOST is not set")
    }

    #[allow(non_snake_case, dead_code)]
    pub fn SERVER_PORT(&self) -> &str {
        CONFIG
            .data
            .get("SERVER_PORT")
            .expect("SERVER_PORT is not set")
    }
}
