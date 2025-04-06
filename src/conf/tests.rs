#[cfg(test)]
mod from_str {
    use libc::SIGTERM;

    use crate::conf::{
        Config,
        proc::{defaults, types},
    };

    #[test]
    fn only_required_values() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"";
        let conf = Config::from_str(conf_str).expect("could not parse config");

        assert_eq!(conf.processes().keys().cloned().collect::<Vec<String>>(), vec!["nginx".to_string()]);
        assert_eq!(conf.processes()["nginx"].processes(), defaults::dflt_processes());
        assert_eq!(conf.processes()["nginx"].umask(), defaults::dflt_umask().mask());
        assert_eq!(conf.processes()["nginx"].autostart(), defaults::dflt_autostart());
        assert_eq!(conf.processes()["nginx"].autorestart().mode(), defaults::dflt_autorestart().mode());
        assert_eq!(conf.processes()["nginx"].exitcodes(), &defaults::dflt_exitcodes());
        assert_eq!(conf.processes()["nginx"].startretries(), defaults::dflt_startretries());
        assert_eq!(conf.processes()["nginx"].starttime(), defaults::dflt_startttime());
        assert_eq!(conf.processes()["nginx"].stopsignals(), &defaults::dflt_stopsignals());
        assert_eq!(conf.processes()["nginx"].stoptime(), defaults::dflt_stoptime());
        assert_eq!(conf.processes()["nginx"].stdout(), "/tmp/nginx.stdout");
        assert_eq!(conf.processes()["nginx"].stderr(), "/tmp/nginx.stderr");
        assert_eq!(conf.processes()["nginx"].env(), &None);
    }

    #[test]
    fn socketpath_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\n";
        assert_eq!(
            Config::from_str(conf_str).expect("could not parse config").socketpath(),
            "/tmp/.taskmaster.sock".to_string()
        )
    }

    #[test]
    fn cmd_nonexisting() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/ngin\"\nworkingdir = \"/tmp\"\n";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn cmd_no_exec_rights() {
        let conf_str = "[processes.nginx]\ncmd = \"Cargo.toml\"\nworkingdir = \"/tmp\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn cmd_is_dir() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr\"\nworkingdir = \"/tmp\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn cmd_is_not_regfile() {
        let conf_str = "[processes.nginx]\ncmd = \"/dev/urandom\"\nworkingdir = \"/tmp\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn cmd_missing() {
        let conf_str = "[processes.nginx]\nworkingdir = \"/tmp\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn args_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\n";
        let expected_vec: Vec<String> = vec![];
        assert_eq!(
            Config::from_str(conf_str).expect("could not parse config").processes()["nginx"].args(),
            &expected_vec
        )
    }

    #[test]
    fn processes_out_of_range() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nprocesses = 256";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn processes_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\n";
        assert_eq!(
            Config::from_str(conf_str).expect("could not parse config string").processes()["nginx"].processes(),
            defaults::dflt_processes()
        );
    }

    #[test]
    fn umask_invalid_char() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\numask = \"098\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn umask_invalid_len() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\numask = \"7777\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn umask_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"";
        assert_eq!(Config::from_str(conf_str).expect("could not parse config").processes()["nginx"].umask(), 0o022);
    }

    #[test]
    fn workingdir_missing() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\n";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn workingdir_nonexisting() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/asdasda\"\n";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn workingdir_is_device() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/dev/urandom\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn workingdir_is_dir() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"";
        assert!(Config::from_str(conf_str).is_ok());
    }

    #[test]
    fn workingdir_is_not_dir() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"Cargo.toml\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn autostart_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"";
        assert!(!Config::from_str(conf_str).expect("could not parse config").processes()["nginx"].autostart());
    }

    #[test]
    fn autorestart_invalid_value() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = maybe";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn autorestart_on_failure_out_of_range() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = on-failure[:256]";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn autorestart_on_failure_malformed_non_alphanumeric() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = on-failure[:a256]";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn autorestart_on_failure_malformed_no_bracket() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = on-failure:256";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn autorestart_on_failure_malformed_no_colon() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = on-failure[256]";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn autorestart_on_failure_success() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = \"on-failure[:5]\"";
        let conf = Config::from_str(conf_str).expect("could not parse config");
        let autores = conf.processes()["nginx"].autorestart();
        assert_eq!(autores.mode(), "on-failure");
        assert_eq!(autores.max_retries(), 5);
    }

    #[test]
    fn exitcodes_out_of_range() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nexitcodes = [2147483649]";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn exitcodes_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\n";
        assert_eq!(
            *Config::from_str(conf_str).expect("could not parse config").processes()["nginx"].exitcodes(),
            vec![0]
        );
    }

    #[test]
    fn startretries_out_of_range() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstartretries = 256";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn startretries_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\n";
        assert_eq!(
            Config::from_str(conf_str).expect("could not parse config").processes()["nginx"].startretries(),
            3
        );
    }

    #[test]
    fn starttime_out_of_range() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstarttime = 70000";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn starttime_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\n";
        assert_eq!(Config::from_str(conf_str).expect("could not parse config").processes()["nginx"].starttime(), 5);
    }

    #[test]
    fn stopsignals_non_existing_freebsd_signal() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstopsignals = [\"ARTHUR\"]";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn stopsignals_with_sig_prefix() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstopsignals = [\"SIGUSR1\"]";
        assert!(Config::from_str(conf_str).is_ok());
    }

    #[test]
    fn stopsignals_without_sig_prefix() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstopsignals = [\"USR1\"]";
        assert!(Config::from_str(conf_str).is_ok());
    }

    #[test]
    fn stopsignals_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\n";
        assert_eq!(
            *Config::from_str(conf_str).expect("could not parse config").processes()["nginx"].stopsignals(),
            vec![types::StopSignal(SIGTERM)]
        );
    }

    #[test]
    fn stoptime_out_of_range() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstoptime  = 256";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn stoptime_default() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\n";
        assert_eq!(Config::from_str(conf_str).expect("could not parse config").processes()["nginx"].stoptime(), 5);
    }

    #[test]
    fn stdout_is_directory() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstdout = /tmp";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn stderr_is_directory() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nstderr = /tmp";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn env_malformed() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nenv = [\"hello\",\"world\"]";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn missing_required_fields() {
        let conf_str = "[processes.nginx]\nworkingdir = \"/tmp\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn invalid_toml_syntax() {
        let conf_str = "[nginx\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn multiple_processes() {
        let conf_str = r#"
            [processes.nginx]
            cmd = "/usr/sbin/nginx"
            workingdir = "/tmp"

            [processes.apache]
            cmd = "/usr/bin/cat"
            workingdir = "/tmp"
        "#;
        let conf = Config::from_str(conf_str).expect("could not parse config");
        assert_eq!(conf.processes().len(), 2);
    }

    #[test]
    fn default_values() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"";
        let conf = Config::from_str(conf_str).expect("could not parse config");
        let proc = conf.processes().get("nginx").unwrap();
        assert_eq!(proc.processes(), defaults::dflt_processes());
        assert_eq!(proc.umask(), defaults::dflt_umask().mask());
    }

    #[test]
    fn invalid_field_values() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nprocesses = -1";
        assert!(Config::from_str(conf_str).is_err());
    }

    #[test]
    fn edge_cases_for_numeric_fields() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nprocesses = 255";
        let conf = Config::from_str(conf_str).expect("could not parse config");
        assert_eq!(conf.processes().get("nginx").unwrap().processes(), 255);
    }

    #[test]
    fn env_valid() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nenv = [[\"KEY\", \"VALUE\"]]";
        let conf = Config::from_str(conf_str).expect("could not parse config");
        let env = conf.processes().get("nginx").unwrap().env();
        assert_eq!(env, &Some(vec![("KEY".to_string(), "VALUE".to_string())]));
    }

    #[test]
    fn autorestart_valid_modes() {
        let conf_str = "[processes.nginx]\ncmd = \"/usr/sbin/nginx\"\nworkingdir = \"/tmp\"\nautorestart = \"always\"";
        let conf = Config::from_str(conf_str).expect("could not parse config");
        assert_eq!(conf.processes().get("nginx").unwrap().autorestart().mode(), "always");
    }
}

#[cfg(test)]
mod from_file {
    use libc::{SIGTERM, SIGUSR1};

    use crate::conf::{
        Config,
        proc::types::{AccessibleDirectory, StopSignal},
    };

    #[test]
    fn invalid_path() {
        assert!(Config::from_file("foobar/../../foo/.bar").is_err());
    }

    #[test]
    fn empty_config() {
        let conf = Config::from_file("./config/empty.toml");
        assert!(conf.is_err());
    }

    #[test]
    fn valid_config_all_fields_set() {
        let conf = Config::from_file("./tests/configs/example.toml").expect("could not parse config");

        assert_eq!(conf.processes().keys().cloned().collect::<Vec<String>>(), vec!["nginx".to_string()]);
        assert_eq!(conf.processes()["nginx"].cmd().path(), "/usr/sbin/nginx");
        assert_eq!(conf.processes()["nginx"].processes(), 1);
        assert_eq!(conf.processes()["nginx"].umask(), 0o022);
        assert_eq!(conf.processes()["nginx"].workingdir(), &AccessibleDirectory::default());
        assert!(conf.processes()["nginx"].autostart());
        assert_eq!(conf.processes()["nginx"].autorestart().mode(), "on-failure");
        assert_eq!(conf.processes()["nginx"].autorestart().max_retries(), 5);
        assert_eq!(conf.processes()["nginx"].exitcodes(), &vec![0, 2]);
        assert_eq!(conf.processes()["nginx"].startretries(), 3);
        assert_eq!(conf.processes()["nginx"].starttime(), 5);
        assert_eq!(conf.processes()["nginx"].stopsignals(), &vec![StopSignal(SIGTERM), StopSignal(SIGUSR1)]);
        assert_eq!(conf.processes()["nginx"].stoptime(), 5);
        assert_eq!(conf.processes()["nginx"].stdout(), ("/tmp/nginx.stdout".to_string()));
        assert_eq!(conf.processes()["nginx"].stderr(), ("/tmp/nginx.stderr".to_string()));
        assert_eq!(
            conf.processes()["nginx"].env(),
            &Some(vec![
                (("STARTED_BY".to_string()), ("abied-ch".to_string())),
                (("ANSWER".to_string()), ("42".to_string()))
            ])
        );
    }
}
