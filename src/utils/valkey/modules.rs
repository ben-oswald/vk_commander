use crate::errors::Error;
use crate::utils::ValkeyClient;

pub struct ModulInfo {
    pub name: String,
    pub version: String,
    pub path: String,
    pub args: String,
}

pub enum Module {
    JSON { modul_info: ModulInfo },
    BloomFilter { modul_info: ModulInfo },
    Unknown { modul_info: ModulInfo },
}

impl Module {
    pub fn load_modules(client: &ValkeyClient) -> Result<Vec<Self>, Error> {
        let response = client.exec("MODULE LIST")?;
        let mut modules: Vec<Self> = Vec::new();
        let mut i = 0;

        while i < response.len() {
            let mut name = String::new();
            let mut version = String::new();
            let mut path = String::new();
            let mut args = String::new();

            while i < response.len() {
                let key = &response[i];
                i += 1;

                if i >= response.len() {
                    break;
                }

                let value = &response[i];
                i += 1;

                match key.as_str() {
                    "name" => name = value.clone(),
                    "ver" => version = value.clone(),
                    "path" => path = value.clone(),
                    "args" => args = value.clone(),
                    _ => {}
                }

                if !name.is_empty() && !version.is_empty() && !args.is_empty() {
                    break;
                }
            }

            if !name.is_empty() {
                let module_info = ModulInfo {
                    name: name.clone(),
                    version,
                    path,
                    args,
                };

                let module = match name.as_str() {
                    "json" => Module::JSON {
                        modul_info: module_info,
                    },
                    "bf" => Module::BloomFilter {
                        modul_info: module_info,
                    },
                    _ => Module::Unknown {
                        modul_info: module_info,
                    },
                };

                modules.push(module);
            }
        }

        Ok(modules)
    }
}
