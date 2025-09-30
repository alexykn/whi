use std::env;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorWhen {
    Auto,
    Never,
    Always,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct Args {
    pub names: Vec<String>,
    pub full: bool,
    pub follow_symlinks: bool,
    pub print0: bool,
    pub quiet: bool,
    pub silent: bool,
    pub one: bool,
    pub show_nonexec: bool,
    pub path_override: Option<String>,
    pub color: ColorWhen,
    pub stat: bool,
    pub index: bool,
}

impl Args {
    pub fn parse() -> Result<Self, String> {
        let mut args = Args {
            names: Vec::new(),
            full: false,
            follow_symlinks: false,
            print0: false,
            quiet: false,
            silent: false,
            one: false,
            show_nonexec: false,
            path_override: None,
            color: ColorWhen::Auto,
            stat: false,
            index: false,
        };

        let args_iter = env::args().skip(1);
        let mut expect_path = false;
        let mut expect_color = false;

        for arg in args_iter {
            if expect_path {
                args.path_override = Some(arg);
                expect_path = false;
                continue;
            }

            if expect_color {
                args.color = Self::parse_color(&arg)?;
                expect_color = false;
                continue;
            }

            Self::process_arg(&mut args, &arg, &mut expect_path, &mut expect_color)?;
        }

        if expect_path {
            return Err("--path requires a value".to_string());
        }
        if expect_color {
            return Err("--color requires a value".to_string());
        }

        Ok(args)
    }

    fn parse_color(val: &str) -> Result<ColorWhen, String> {
        match val {
            "auto" => Ok(ColorWhen::Auto),
            "never" => Ok(ColorWhen::Never),
            "always" => Ok(ColorWhen::Always),
            _ => Err(format!("Invalid color value: {val}")),
        }
    }

    fn process_arg(
        args: &mut Args,
        arg: &str,
        expect_path: &mut bool,
        expect_color: &mut bool,
    ) -> Result<(), String> {
        match arg {
            "-f" | "--full" => args.full = true,
            "-i" | "--index" => args.index = true,
            "-l" | "-L" | "--follow-symlinks" => args.follow_symlinks = true,
            "-o" | "--one" => args.one = true,
            "-0" | "--print0" => args.print0 = true,
            "-q" | "--quiet" => args.quiet = true,
            "-s" | "--stat" => args.stat = true,
            "--silent" => args.silent = true,
            "--show-nonexec" => args.show_nonexec = true,
            "--path" => *expect_path = true,
            "--color" => *expect_color = true,
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            s if s.starts_with("--path=") => {
                args.path_override = Some(s.trim_start_matches("--path=").to_string());
            }
            s if s.starts_with("--color=") => {
                let val = s.trim_start_matches("--color=");
                args.color = Self::parse_color(val)?;
            }
            s if s.starts_with('-') && !s.starts_with("--") && s.len() > 2 => {
                Self::parse_combined_flags(args, s)?;
            }
            s if s.starts_with('-') => {
                return Err(format!("Unknown option: {s}"));
            }
            _ => args.names.push(arg.to_string()),
        }
        Ok(())
    }

    fn parse_combined_flags(args: &mut Args, s: &str) -> Result<(), String> {
        // Handle combined short flags like -ef, -eL, -efL
        for ch in s[1..].chars() {
            match ch {
                'f' => args.full = true,
                'i' => args.index = true,
                'l' | 'L' => args.follow_symlinks = true,
                'o' => args.one = true,
                's' => args.stat = true,
                '0' => args.print0 = true,
                'q' => args.quiet = true,
                'h' => {
                    print_help();
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown flag: -{ch}")),
            }
        }
        Ok(())
    }
}

fn print_help() {
    println!(
        "whicha - list all PATH hits and explain why one wins

USAGE:
    whicha [FLAGS] [OPTIONS] <NAME>...
    whicha [FLAGS] [OPTIONS]          # names from stdin

FLAGS:
    -f, --full             Show full PATH directory listing
    -i, --index            Show PATH index next to each entry
    -l, -L, --follow-symlinks
                           Resolve and show canonical targets
    -s, --stat             Include inode/device/mtime/size metadata
    -0, --print0           NUL-separated output for xargs
    -q, --quiet            Suppress non-fatal stderr warnings
        --silent           Print nothing to stderr, use exit codes only
    -o, --one              Only print the first match per name
        --show-nonexec     Also list files that exist but aren't executable
    -h, --help             Print help information

OPTIONS:
        --path <PATH>      Override environment PATH string
        --color <WHEN>     Colorize output: auto, never, always [default: auto]

EXIT CODES:
    0  All names found
    1  At least one not found
    2  Usage error
    3  I/O or environment error"
    );
}
