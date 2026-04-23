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
    let indexed: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{i}\t{s}"))
        .collect();

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

#[cfg(test)]
mod tests {
    use super::*;
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
}
