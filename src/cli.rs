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
    pub all: bool,
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
    pub move_indices: Option<(usize, usize)>,
    pub swap_indices: Option<(usize, usize)>,
    pub prefer_target: Option<(String, usize)>,
    pub init_shell: Option<String>,
    pub clean: bool,
    pub delete_indices: Vec<usize>,
    pub save_shell: Option<Option<String>>, // None = not used, Some(None) = current, Some(Some(x)) = specific
    pub diff_shell: Option<Option<String>>, // None = not used, Some(None) = current, Some(Some(x)) = specific
    pub diff_full: bool,
}

struct ParseState {
    expect_path: bool,
    expect_color: bool,
    expect_move_from: bool,
    expect_swap_from: bool,
    expect_prefer_name: bool,
    expect_delete: bool,
}

impl Args {
    #[allow(clippy::too_many_lines)]
    pub fn parse() -> Result<Self, String> {
        let mut args = Args {
            names: Vec::new(),
            all: false,
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
            move_indices: None,
            swap_indices: None,
            prefer_target: None,
            init_shell: None,
            clean: false,
            delete_indices: Vec::new(),
            save_shell: None,
            diff_shell: None,
            diff_full: false,
        };

        let args_vec: Vec<String> = env::args().skip(1).collect();

        // Handle 'init' subcommand early
        if let Some(idx) = args_vec.iter().position(|a| a == "init") {
            if idx + 1 >= args_vec.len() {
                return Err("init requires a shell argument (bash, zsh, fish)".to_string());
            }
            args.init_shell = Some(args_vec[idx + 1].clone());
            return Ok(args);
        }

        // Handle 'save' subcommand
        if let Some(idx) = args_vec.iter().position(|a| a == "save") {
            if idx + 1 < args_vec.len() {
                let next_arg = &args_vec[idx + 1];
                if !next_arg.starts_with('-') {
                    // Has argument: save <shell>
                    args.save_shell = Some(Some(next_arg.clone()));
                } else {
                    // No argument: save (current shell)
                    args.save_shell = Some(None);
                }
            } else {
                // No argument: save (current shell)
                args.save_shell = Some(None);
            }
            return Ok(args);
        }

        // Handle 'diff' subcommand
        if let Some(idx) = args_vec.iter().position(|a| a == "diff") {
            if idx + 1 < args_vec.len() {
                let next_arg = &args_vec[idx + 1];
                if next_arg == "full" {
                    // diff full: show full diff with M and U indicators
                    args.diff_full = true;
                    args.diff_shell = Some(None); // Use current shell
                } else if !next_arg.starts_with('-') {
                    // Has argument: diff <shell>
                    args.diff_shell = Some(Some(next_arg.clone()));
                } else {
                    // No argument: diff (current shell)
                    args.diff_shell = Some(None);
                }
            } else {
                // No argument: diff (current shell)
                args.diff_shell = Some(None);
            }
            return Ok(args);
        }

        let mut state = ParseState {
            expect_path: false,
            expect_color: false,
            expect_move_from: false,
            expect_swap_from: false,
            expect_prefer_name: false,
            expect_delete: false,
        };
        let mut move_from: Option<usize> = None;
        let mut swap_from: Option<usize> = None;
        let mut prefer_name: Option<String> = None;

        for arg in args_vec {
            if state.expect_path {
                args.path_override = Some(arg);
                state.expect_path = false;
                continue;
            }

            if state.expect_color {
                args.color = Self::parse_color(&arg)?;
                state.expect_color = false;
                continue;
            }

            if state.expect_move_from {
                let from = arg
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid move source index: {arg}"))?;
                move_from = Some(from);
                state.expect_move_from = false;
                continue;
            }

            if let Some(from) = move_from {
                let to = arg
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid move destination index: {arg}"))?;
                args.move_indices = Some((from, to));
                move_from = None;
                continue;
            }

            if state.expect_swap_from {
                let idx1 = arg
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid swap first index: {arg}"))?;
                swap_from = Some(idx1);
                state.expect_swap_from = false;
                continue;
            }

            if let Some(idx1) = swap_from {
                let idx2 = arg
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid swap second index: {arg}"))?;
                args.swap_indices = Some((idx1, idx2));
                swap_from = None;
                continue;
            }

            if state.expect_prefer_name {
                prefer_name = Some(arg);
                state.expect_prefer_name = false;
                continue;
            }

            if let Some(name) = prefer_name {
                let idx = arg
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid prefer index: {arg}"))?;
                args.prefer_target = Some((name, idx));
                prefer_name = None;
                continue;
            }

            if state.expect_delete {
                // Try to parse as index
                if let Ok(idx) = arg.parse::<usize>() {
                    args.delete_indices.push(idx);
                    continue;
                } else {
                    // Not a number, stop expecting delete indices
                    state.expect_delete = false;
                    // Fall through to process this arg normally
                }
            }

            Self::process_arg(&mut args, &arg, &mut state)?;
        }

        if state.expect_path {
            return Err("--path requires a value".to_string());
        }
        if state.expect_color {
            return Err("--color requires a value".to_string());
        }
        if state.expect_move_from || move_from.is_some() {
            return Err("--move requires two indices: FROM TO".to_string());
        }
        if state.expect_swap_from || swap_from.is_some() {
            return Err("--swap requires two indices: IDX1 IDX2".to_string());
        }
        if state.expect_prefer_name || prefer_name.is_some() {
            return Err("--prefer requires NAME and INDEX".to_string());
        }
        if state.expect_delete && args.delete_indices.is_empty() {
            return Err("--delete requires at least one index".to_string());
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

    fn process_arg(args: &mut Args, arg: &str, state: &mut ParseState) -> Result<(), String> {
        match arg {
            "-a" | "--all" => args.all = true,
            "-f" | "--full" => args.full = true,
            "-i" | "--index" => args.index = true,
            "-l" | "-L" | "--follow-symlinks" => args.follow_symlinks = true,
            "--move" => state.expect_move_from = true,
            "--swap" => state.expect_swap_from = true,
            "--prefer" => state.expect_prefer_name = true,
            "-c" | "--clean" => args.clean = true,
            "-d" | "--delete" => state.expect_delete = true,
            "-o" | "--one" => args.one = true,
            "-0" | "--print0" => args.print0 = true,
            "-q" | "--quiet" => args.quiet = true,
            "-s" | "--stat" => args.stat = true,
            "--silent" => args.silent = true,
            "--show-nonexec" => args.show_nonexec = true,
            "--path" => state.expect_path = true,
            "--color" => state.expect_color = true,
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
        // Handle combined short flags like -af, -ail, -aifL
        for ch in s[1..].chars() {
            match ch {
                'a' => args.all = true,
                'c' => args.clean = true,
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
        "whi - magically simple PATH management

USAGE:
    whi [FLAGS] [OPTIONS] <NAME>...
    whi [FLAGS] [OPTIONS]            # names from stdin
    whi --move <FROM> <TO>           # reorder PATH
    whi --swap <IDX1> <IDX2>         # swap PATH entries
    whi --prefer <NAME> <INDEX>      # prefer executable at INDEX
    whi --clean                      # remove duplicate PATH entries
    whi --delete <INDEX>...          # delete PATH entries at indices
    whi save [SHELL]                 # persist PATH changes
    whi diff [SHELL]                 # show PATH differences vs saved
    whi init <SHELL>                 # output shell integration

FLAGS:
    -a, --all              Show all PATH matches (default: only winner)
    -f, --full             Show all matches + full PATH listing (implies -a)
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

PATH MANIPULATION:
        --move <FROM> <TO> Move PATH entry from index FROM to index TO
        --swap <IDX1> <IDX2>
                           Swap PATH entries at indices IDX1 and IDX2
        --prefer <NAME> <INDEX>
                           Make executable NAME at INDEX win by moving it
                           just before the current winner (minimal change)
    -c, --clean            Remove duplicate PATH entries (keeps first occurrence)
    -d, --delete <INDEX>...
                           Delete PATH entries at one or more indices

OPTIONS:
        --path <PATH>      Override environment PATH string
        --color <WHEN>     Colorize output: auto, never, always [default: auto]

PERSISTENCE:
    whi save           Save current PATH for current shell (auto-detected)
    whi save bash      Save current PATH for bash
    whi save zsh       Save current PATH for zsh
    whi save fish      Save current PATH for fish
    whi save all       Save current PATH for all shells

    whi diff           Compare current vs saved PATH for current shell
    whi diff bash      Compare for bash
    whi diff zsh       Compare for zsh
    whi diff fish      Compare for fish

    After 'whi save', PATH changes persist across new terminal sessions.
    The saved PATH is automatically loaded from your shell config file.
    Use 'whi diff' to see what changed before saving.

SHELL INTEGRATION:
    whi init bash       Output bash integration code
    whi init zsh        Output zsh integration code
    whi init fish       Output fish integration code

    Add to your shell config:
        bash/zsh: eval \"$(whi init bash)\" or eval \"$(whi init zsh)\"
        fish:     whi init fish | source

    Provides shell commands to manipulate PATH in current shell:
        whim 10 1      # Move PATH entry 10 to position 1
        whis 10 41     # Swap PATH entries 10 and 41
        whip cargo 50  # Make cargo at index 50 the winner
        whic           # Clean duplicate PATH entries
        whid 4         # Delete PATH entry at index 4
        whid 5 16 7    # Delete PATH entries at indices 5, 16, and 7
        whia cargo     # Show all cargo matches with indices (whi -ia)
        whii           # Show all PATH entries with indices (whi -i)

EXIT CODES:
    0  All names found
    1  At least one not found
    2  Usage error
    3  I/O or environment error"
    );
}
