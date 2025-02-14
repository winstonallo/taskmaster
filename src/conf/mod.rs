use std::{collections::HashMap, fs};

use proc::ProcessConfig;

pub mod proc;

pub struct Config {
    processes: HashMap<String, ProcessConfig>,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, String> {
        let conf_str = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(err) => {
                return Err(format!("could not read config at path '{path}' to into string: {err}"));
            }
        };

        let mut procs: HashMap<String, ProcessConfig> = match toml::from_str(&conf_str) {
            Ok(procs) => procs,
            Err(err) => {
                return Err(format!("could not parse config at '{path}': {err}"));
            }
        };

        for (proc_name, proc) in &mut procs {
            if proc.stdout().is_empty() {
                proc.set_stdout(&format!("/tmp/{proc_name}.stdout"));
            }
            if proc.stderr().is_empty() {
                proc.set_stderr(&format!("/tmp/{proc_name}.stderr"));
            }
        }

        Ok(Config { processes: procs })
    }

    #[cfg(test)]
    pub fn from_str(config: &str) -> Result<Self, String> {
        let mut procs: HashMap<String, ProcessConfig> = match toml::from_str(&config) {
            Ok(procs) => procs,
            Err(err) => {
                return Err(format!("could not parse config string: {err}"));
            }
        };

        for (proc_name, proc) in &mut procs {
            if proc.stdout().is_empty() {
                proc.set_stdout(&format!("/tmp/{proc_name}.stdout"));
            }
            if proc.stderr().is_empty() {
                proc.set_stderr(&format!("/tmp/{proc_name}.stderr"));
            }
        }

        Ok(Config { processes: procs })
    }

    pub fn get_processes(&self) -> &HashMap<String, ProcessConfig> {
        &self.processes
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_file_invalid_path() {
        assert!(Config::from_file("foobar/../../foo/.bar").is_err());
    }

    #[test]
    fn from_file_empty_config() {
        let conf = Config::from_file("./config/empty.toml");
        assert!(conf.is_err());
    }

    #[test]
    fn from_file_valid_config_all_fields_set() {
        let conf = Config::from_file("./tests/configs/example.toml").expect("could not parse config");

        assert_eq!(conf.get_processes().keys().cloned().collect::<Vec<String>>(), vec!["nginx".to_string()]);
        assert_eq!(conf.get_processes()["nginx"].cmd(), "/usr/sbin/nginx");
        assert_eq!(conf.get_processes()["nginx"].processes(), 1);
        assert_eq!(conf.get_processes()["nginx"].umask(), "022");
        assert_eq!(conf.get_processes()["nginx"].workingdir(), "/tmp");
        assert_eq!(conf.get_processes()["nginx"].autostart(), true);
        assert_eq!(conf.get_processes()["nginx"].autorestart().mode(), "on-failure");
        assert_eq!(conf.get_processes()["nginx"].autorestart().max_retries(), Some(5));
        assert_eq!(conf.get_processes()["nginx"].exitcodes(), &vec![0, 2]);
        assert_eq!(conf.get_processes()["nginx"].startretries(), 3);
        assert_eq!(conf.get_processes()["nginx"].starttime(), 5);
        assert_eq!(
            conf.get_processes()["nginx"].stopsignals(),
            &vec![
                proc::deserializers::stopsignal::StopSignal::SigTerm,
                proc::deserializers::stopsignal::StopSignal::SigUsr1
            ]
        );
        assert_eq!(conf.get_processes()["nginx"].stoptime(), 5);
        assert_eq!(conf.get_processes()["nginx"].stdout(), String::from("/tmp/nginx.stdout"));
        assert_eq!(conf.get_processes()["nginx"].stderr(), String::from("/tmp/nginx.stderr"));
        assert_eq!(
            conf.get_processes()["nginx"].env(),
            &Some(vec![
                (String::from("STARTED_BY"), String::from("abied-ch")),
                (String::from("ANSWER"), String::from("42"))
            ])
        );
    }

    #[test]
    fn from_str_only_required_values() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"";
        let conf = Config::from_str(&conf_str).expect("could not parse config");

        assert_eq!(conf.get_processes().keys().cloned().collect::<Vec<String>>(), vec!["nginx".to_string()]);
        assert_eq!(conf.get_processes()["nginx"].processes(), proc::defaults::dflt_processes());
        assert_eq!(conf.get_processes()["nginx"].umask(), proc::defaults::dflt_umask());
        assert_eq!(conf.get_processes()["nginx"].autostart(), proc::defaults::dflt_autostart());
        assert_eq!(conf.get_processes()["nginx"].autorestart().mode(), proc::defaults::dflt_autorestart().mode());
        assert_eq!(
            conf.get_processes()["nginx"].autorestart().max_retries(),
            proc::defaults::dflt_autorestart().max_retries()
        );
        assert_eq!(conf.get_processes()["nginx"].exitcodes(), &proc::defaults::dflt_exitcodes());
        assert_eq!(conf.get_processes()["nginx"].startretries(), proc::defaults::dflt_startretries());
        assert_eq!(conf.get_processes()["nginx"].starttime(), proc::defaults::dflt_startttime());
        assert_eq!(conf.get_processes()["nginx"].stopsignals(), &proc::defaults::dflt_stopsignals());
        assert_eq!(conf.get_processes()["nginx"].stoptime(), proc::defaults::dflt_stoptime());
        assert_eq!(conf.get_processes()["nginx"].stdout(), "/tmp/nginx.stdout");
        assert_eq!(conf.get_processes()["nginx"].stderr(), "/tmp/nginx.stderr");
        assert_eq!(conf.get_processes()["nginx"].env(), &None);
    }

    #[test]
    fn from_str_processes_out_of_range() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nprocesses = 256";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_autorestart_on_failure_out_of_range() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = on-failure[:256]";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_autorestart_on_failure_malformed_non_alphanumeric() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = on-failure[:a256]";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_autorestart_on_failure_malformed_no_bracket() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = on-failure:256";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_autorestart_on_failure_malformed_no_colon() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = on-failure[256]";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_exitcodes_out_of_range() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nexitcodes = [256]";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_startretries_out_of_range() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstartretries = 256";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_starttime_out_of_range() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstarttime = 70000";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_stopsignals_non_existing_freebsd_signal() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstopsignals = [\"ARTHUR\"]";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_stoptime_out_of_range() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstoptime  = 256";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_env_malformed() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nenv = [\"hello\",\"world\"]";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_missing_required_fields() {
        let conf_str = "[nginx]\nworkingdir = \"/tmp\"";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_invalid_toml_syntax() {
        let conf_str = "[nginx\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_extra_fields() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nextra_field = \"value\"";
        let conf = Config::from_str(&conf_str).expect("could not parse config");
        assert_eq!(conf.get_processes().len(), 1);
    }

    #[test]
    fn from_str_multiple_processes() {
        let conf_str = r#"
            [nginx]
            cmd = "/usr/sbin/nginx"
            workingdir = "/tmp"

            [apache]
            cmd = "/usr/sbin/apache2"
            workingdir = "/var/www"
        "#;
        let conf = Config::from_str(&conf_str).expect("could not parse config");
        assert_eq!(conf.get_processes().len(), 2);
    }

    #[test]
    fn from_str_default_values() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"";
        let conf = Config::from_str(&conf_str).expect("could not parse config");
        let proc = conf.get_processes().get("nginx").unwrap();
        assert_eq!(proc.processes(), proc::defaults::dflt_processes());
        assert_eq!(proc.umask(), proc::defaults::dflt_umask());
    }

    #[test]
    fn from_str_invalid_field_values() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nprocesses = -1";
        assert!(Config::from_str(&conf_str).is_err());
    }

    #[test]
    fn from_str_edge_cases_for_numeric_fields() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nprocesses = 255";
        let conf = Config::from_str(&conf_str).expect("could not parse config");
        assert_eq!(conf.get_processes().get("nginx").unwrap().processes(), 255);
    }

    #[test]
    fn from_str_env_valid() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nenv = [[\"KEY\", \"VALUE\"]]";
        let conf = Config::from_str(&conf_str).expect("could not parse config");
        let env = conf.get_processes().get("nginx").unwrap().env();
        assert_eq!(env, &Some(vec![("KEY".to_string(), "VALUE".to_string())]));
    }

    #[test]
    fn from_str_autorestart_valid_modes() {
        let conf_str = "[nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = \"always\"";
        let conf = Config::from_str(&conf_str).expect("could not parse config");
        assert_eq!(conf.get_processes().get("nginx").unwrap().autorestart().mode(), "always");
    }
}
