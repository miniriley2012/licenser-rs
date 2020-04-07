use std::fmt::Formatter;
use std::io::Write;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct License {
    key: String,
    name: String,
    url: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    implementation: String,
    #[serde(default)]
    body: String,
}

impl std::fmt::Display for License {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str("Name: ")?;
        f.write_str(self.name.as_str())?;
        f.write_str("\n\n")?;
        f.write_str("Description: ")?;
        f.write_str(self.description.as_str())?;
        f.write_str("\n\n")?;
        f.write_str("Implementation: ")?;
        f.write_str(self.implementation.as_str())?;
        f.write_str("\n\n")?;
        f.write_str(self.body.as_str())?;
        f.write_str("\n")
    }
}

struct Licenser {
    licenses: Vec<License>
}

impl Licenser {
    fn request_license(url: &str) -> Result<License, reqwest::Error> {
        Ok(reqwest::blocking::Client::new()
            .get(url)
            .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
            .header(reqwest::header::USER_AGENT, "licenser")
            .send()?
            .json()?)
    }

    fn rewrite_licenses(licenses: Vec<License>) -> Result<Vec<License>, Box<dyn std::error::Error>> {
        let mut new = licenses.to_vec();
        for i in 0..new.len() {
            new[i] = Licenser::request_license(new[i].url.as_str())?;
            println!("Downloaded {}.", new[i].name);
        }
        Ok(new)
    }

    fn init_license_db(path: &std::path::Path) -> Result<Vec<License>, Box<dyn std::error::Error>> {
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        let licenses = Licenser::rewrite_licenses(Licenser::request_licenses()?)?;
        serde_json::to_writer(writer, &licenses)?;
        Ok(licenses)
    }

    // BUG: function will return an error if licenses.db is not in the expected format
    fn new() -> Result<Licenser, Box<dyn std::error::Error>> {
        let db_path = dirs::home_dir().unwrap().join(".licenser/licenses.db");

        Ok(Licenser {
            licenses: if db_path.exists() {
                let file = std::fs::File::open(db_path)?;
                let reader = std::io::BufReader::new(file);
                serde_json::from_reader(reader)?
            } else {
                std::fs::create_dir_all(db_path.parent().unwrap())?;
                match Licenser::init_license_db(db_path.as_path()) {
                    Ok(licenses) => licenses,
                    Err(e) => {
                        std::fs::remove_file(db_path)?;
                        return Err(e);
                    }
                }
            }
        })
    }

    fn get_license(self, name: &str) -> Option<License> {
        for license in self.licenses {
            if license.key == name {
                return Some(license);
            }
        }
        None
    }

    fn request_licenses() -> Result<Vec<License>, reqwest::Error> {
        Ok(reqwest::blocking::Client::new()
            .get("https://api.github.com/licenses")
            .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
            .header(reqwest::header::USER_AGENT, "licenser")
            .send()?
            .json()?)
    }
}

fn main() {
    let licenser = Licenser::new().unwrap();
    let mut names: Vec<&str> = vec![];
    for license in &licenser.licenses {
        names.push(license.key.as_str())
    }

    let licenses_arg = clap::Arg::with_name("license")
        .possible_values(names.as_slice())
        .required(true);

    let matches = clap::App::new("licenser")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .author("Riley Quinn")
        .version("0.0.1")
        .about("Licenser creates license files for your projects.")
        .subcommand(clap::SubCommand::with_name("new")
            .arg(&licenses_arg)
            .arg(clap::Arg::from_usage("-o --output=[FILE] 'output file for license'")
                .default_value("LICENSE"))
            .usage("Creates a new license file.")
        )
        .subcommand(clap::SubCommand::with_name("show")
            .arg(&licenses_arg)
            .arg_from_usage("--body-only 'only show the licence's text'")
        )

        .get_matches();

    let sub = matches.subcommand();
    match sub.0 {
        "new" => {
            let path = std::path::Path::new(sub.1.unwrap().value_of("output").unwrap());
            let mut file = std::fs::File::create(path).unwrap();
            file.write_all(licenser.get_license(sub.1.unwrap().value_of("license").unwrap()).unwrap().body.as_bytes()).unwrap();
            println!("Created {}", path.display());
        }
        "show" => {
            let license = licenser.get_license(sub.1.unwrap().value_of("license").unwrap()).unwrap();
            if sub.1.unwrap().is_present("body-only") {
                println!("{}", license.body);
            } else {
                println!("{}", license);
            }
        }
        _ => ()
    };
}
