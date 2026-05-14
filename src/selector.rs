use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn is_fzf_available() -> bool {
    Command::new("fzf")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

pub fn select(items: &[String]) -> Result<Option<usize>, String> {
    // Test escape hatch: BRANCH_SELECT_FIRST=N selects the Nth item (1-based) without interaction
    if let Ok(val) = std::env::var("BRANCH_SELECT_FIRST") {
        let n: usize = val.parse().unwrap_or(0);
        return Ok(if n > 0 && n <= items.len() {
            Some(n - 1)
        } else {
            None
        });
    }
    if is_fzf_available() {
        select_with_fzf(items)
    } else {
        select_builtin(items)
    }
}

fn select_with_fzf(items: &[String]) -> Result<Option<usize>, String> {
    // Prefix each item with its 0-based index and a tab.
    // fzf displays from field 2 onwards (hiding the index), but outputs the full line.
    let mut indexed: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{i}\t{s}"))
        .collect();

    if should_reverse_input_for_fzf() {
        indexed.reverse();
    }

    let mut child = Command::new("fzf")
        .args(["--ansi", "--delimiter=\t", "--with-nth=2.."])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(mut stdin) = child.stdin.take() {
        for line in &indexed {
            writeln!(stdin, "{line}").map_err(|e| e.to_string())?;
        }
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;

    if output.status.success() {
        let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let idx: usize = line
            .split('\t')
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("Failed to parse fzf output index from: {line}"))?;
        Ok(Some(idx))
    } else {
        Ok(None)
    }
}

fn select_builtin(items: &[String]) -> Result<Option<usize>, String> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::Select;

    Select::with_theme(&ColorfulTheme::default())
        .items(items)
        .default(0)
        .interact_opt()
        .map_err(|e| e.to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FzfLayout {
    Default,
    Reverse,
    ReverseList,
}

fn should_reverse_input_for_fzf() -> bool {
    let mut config = FzfConfig {
        layout: FzfLayout::Default,
        tac: false,
    };

    if let Ok(path) = std::env::var("FZF_DEFAULT_OPTS_FILE") {
        if let Ok(contents) = fs::read_to_string(path) {
            apply_fzf_options(&mut config, &contents);
        }
    }

    if let Ok(opts) = std::env::var("FZF_DEFAULT_OPTS") {
        apply_fzf_options(&mut config, &opts);
    }

    matches!(config.layout, FzfLayout::Default) ^ config.tac
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FzfConfig {
    layout: FzfLayout,
    tac: bool,
}

fn apply_fzf_options(config: &mut FzfConfig, options: &str) {
    let tokens = tokenize_fzf_options(options);
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "--reverse" => config.layout = FzfLayout::Reverse,
            "--tac" => config.tac = true,
            "--no-tac" => config.tac = false,
            "--layout" => {
                if let Some(layout) = tokens.get(i + 1).and_then(|value| parse_fzf_layout(value)) {
                    config.layout = layout;
                    i += 1;
                }
            }
            token => {
                if let Some(value) = token.strip_prefix("--layout=") {
                    if let Some(layout) = parse_fzf_layout(value) {
                        config.layout = layout;
                    }
                }
            }
        }
        i += 1;
    }
}

fn parse_fzf_layout(value: &str) -> Option<FzfLayout> {
    match value {
        "default" => Some(FzfLayout::Default),
        "reverse" => Some(FzfLayout::Reverse),
        "reverse-list" => Some(FzfLayout::ReverseList),
        _ => None,
    }
}

fn tokenize_fzf_options(options: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for line in options.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }

        let mut current = String::new();
        let mut quote = None;
        let mut escaped = false;

        for ch in line.chars() {
            if escaped {
                current.push(ch);
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '"' | '\'' => {
                    if quote == Some(ch) {
                        quote = None;
                    } else if quote.is_none() {
                        quote = Some(ch);
                    } else {
                        current.push(ch);
                    }
                }
                c if c.is_whitespace() && quote.is_none() => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    // Mutex to ensure only one test manipulates BRANCH_SELECT_FIRST at a time
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn is_fzf_available_does_not_panic() {
        let _ = is_fzf_available();
    }

    #[test]
    fn select_returns_first_item_index_when_env_set_to_1() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("BRANCH_SELECT_FIRST", "1");
        let items = vec!["main".to_string()];
        let result = select(&items);
        std::env::remove_var("BRANCH_SELECT_FIRST");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(0));
    }

    #[test]
    fn select_returns_correct_index_for_second_item() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("BRANCH_SELECT_FIRST", "2");
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = select(&items);
        std::env::remove_var("BRANCH_SELECT_FIRST");
        assert_eq!(result.unwrap(), Some(1)); // 1-based "2" → 0-based index 1
    }

    #[test]
    fn select_returns_none_when_env_index_out_of_range() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("BRANCH_SELECT_FIRST", "5");
        let items = vec!["a".to_string(), "b".to_string()];
        let result = select(&items);
        std::env::remove_var("BRANCH_SELECT_FIRST");
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn default_layout_reverses_input_for_fzf() {
        let mut config = FzfConfig {
            layout: FzfLayout::Default,
            tac: false,
        };
        apply_fzf_options(&mut config, "");
        assert!(matches!(config.layout, FzfLayout::Default));
        assert!(matches!(config.layout, FzfLayout::Default) ^ config.tac);
    }

    #[test]
    fn reverse_layout_keeps_input_order_for_fzf() {
        let mut config = FzfConfig {
            layout: FzfLayout::Default,
            tac: false,
        };
        apply_fzf_options(&mut config, "--layout=reverse");
        assert!(!matches!(config.layout, FzfLayout::Default) ^ config.tac);
    }

    #[test]
    fn reverse_list_layout_keeps_input_order_for_fzf() {
        let mut config = FzfConfig {
            layout: FzfLayout::Default,
            tac: false,
        };
        apply_fzf_options(&mut config, "--layout reverse-list");
        assert!(!matches!(config.layout, FzfLayout::Default) ^ config.tac);
    }

    #[test]
    fn tac_flips_the_reversal_rule_for_fzf() {
        let mut config = FzfConfig {
            layout: FzfLayout::Default,
            tac: false,
        };
        apply_fzf_options(&mut config, "--tac");
        assert!(!matches!(config.layout, FzfLayout::Default) ^ config.tac);

        let mut reverse_config = FzfConfig {
            layout: FzfLayout::Default,
            tac: false,
        };
        apply_fzf_options(&mut reverse_config, "--layout=reverse --tac");
        assert!(!matches!(reverse_config.layout, FzfLayout::Default));
        assert!(reverse_config.tac);
        assert!(matches!(reverse_config.layout, FzfLayout::Default) ^ reverse_config.tac);
    }

    #[test]
    fn opts_file_is_applied_before_env_opts() {
        let _guard = ENV_LOCK.lock().unwrap();
        let path = std::env::temp_dir().join("branch-fzf-opts-test");
        fs::write(&path, "--layout=default\n").unwrap();

        std::env::set_var("FZF_DEFAULT_OPTS_FILE", &path);
        std::env::set_var("FZF_DEFAULT_OPTS", "--layout=reverse");

        assert!(!should_reverse_input_for_fzf());

        std::env::remove_var("FZF_DEFAULT_OPTS_FILE");
        std::env::remove_var("FZF_DEFAULT_OPTS");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn tokenize_fzf_options_skips_comment_lines() {
        let tokens =
            tokenize_fzf_options("# comment\n--layout=reverse-list\n--color=bg:#112233 --tac");
        assert_eq!(
            tokens,
            vec!["--layout=reverse-list", "--color=bg:#112233", "--tac"]
        );
    }
}
