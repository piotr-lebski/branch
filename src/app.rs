use crate::cli::Cli;
use crate::git::Worktree;

#[derive(Clone)]
pub(crate) enum Item {
    Header,
    Branch { name: String, is_current: bool },
    Worktree { name: String, path: String },
}

pub(crate) struct DisplayList {
    pub items: Vec<Item>,
    pub display: Vec<String>,
}

pub(crate) fn build_display_list(
    branches: Vec<(String, bool)>,
    worktrees: Vec<Worktree>,
) -> DisplayList {
    let mut items = Vec::new();
    let mut display = Vec::new();

    let branch_header = format!("\x1b[2m── Branches {}\x1b[0m", "─".repeat(40));
    items.push(Item::Header);
    display.push(branch_header);

    for (name, is_current) in branches {
        let d = if is_current {
            format!("* {name}")
        } else {
            format!("  {name}")
        };
        items.push(Item::Branch { name, is_current });
        display.push(d);
    }

    if !worktrees.is_empty() {
        let wt_header = format!("\x1b[2m── Worktrees {}\x1b[0m", "─".repeat(39));
        items.push(Item::Header);
        display.push(wt_header);

        let name_width = worktrees.iter().map(|w| w.name.len()).max().unwrap_or(0);
        for wt in worktrees {
            let d = format!("  {:<width$}  {}", wt.name, wt.path, width = name_width);
            items.push(Item::Worktree {
                name: wt.name,
                path: wt.path,
            });
            display.push(d);
        }
    }

    DisplayList { items, display }
}

pub fn run(cli: Cli) -> Result<(), String> {
    if let Some(shell_name) = cli.init {
        let shell = if shell_name == "auto" {
            crate::init::detect_shell()?
        } else {
            crate::init::parse_shell(&shell_name)?
        };
        print!("{}", crate::init::snippet(shell));
        return Ok(());
    }

    loop {
        let branches: Vec<(String, bool)> = if cli.remote {
            crate::git::list_remote_branches(None)?
                .into_iter()
                .map(|name| (name, false))
                .collect()
        } else {
            crate::git::list_local_branches(None)?
        };

        let worktrees = crate::git::list_worktrees(None)?;

        if branches.is_empty() && worktrees.is_empty() {
            eprintln!("No branches or worktrees found.");
            return Ok(());
        }

        let dl = build_display_list(branches, worktrees);

        let selected_idx = match crate::selector::select(&dl.display)? {
            None => return Ok(()),
            Some(idx) => idx,
        };

        if matches!(dl.items[selected_idx], Item::Header) {
            continue;
        }

        let actions = &["Checkout / cd", "Delete", "Cancel"];
        let action_idx = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .items(actions)
            .default(0)
            .interact_opt()
            .map_err(|e| e.to_string())?;

        match action_idx {
            None | Some(2) => continue,
            Some(0) => match &dl.items[selected_idx] {
                Item::Branch { name, .. } => {
                    let checkout_name = if cli.remote {
                        name.split_once('/').map(|x| x.1).unwrap_or(name.as_str())
                    } else {
                        name.as_str()
                    };
                    crate::git::checkout_branch(checkout_name, None)?;
                    return Ok(());
                }
                Item::Worktree { path, .. } => {
                    println!("{path}");
                    return Ok(());
                }
                Item::Header => unreachable!(),
            },
            Some(1) => {
                handle_delete(&dl.items[selected_idx])?;
            }
            _ => unreachable!(),
        }
    }
}

fn delete_branch_with_prompt(name: &str) -> Result<(), String> {
    match crate::git::delete_branch(name, false, None) {
        Ok(()) => eprintln!("Deleted branch: {name}"),
        Err(e) if e.contains("not fully merged") => {
            let force = dialoguer::Confirm::new()
                .with_prompt(format!(
                    "Branch '{name}' is not fully merged. Force delete?"
                ))
                .default(false)
                .interact()
                .map_err(|e| e.to_string())?;
            if force {
                crate::git::delete_branch(name, true, None)?;
                eprintln!("Force deleted branch: {name}");
            }
        }
        Err(e) => eprintln!("Error deleting branch: {e}"),
    }
    Ok(())
}

fn can_force_remove_worktree(error: &str) -> bool {
    error.contains("use --force to delete it")
        || error.contains("contains modified or untracked files")
}

fn remove_worktree_with_prompt(name: &str, path: &str) -> Result<(), String> {
    match crate::git::remove_worktree(path, false, None) {
        Ok(()) => {
            eprintln!("Removed worktree: {path}");
            let also_delete_branch = dialoguer::Confirm::new()
                .with_prompt(format!("Also delete branch '{name}'?"))
                .default(false)
                .interact()
                .map_err(|e| e.to_string())?;
            if also_delete_branch {
                delete_branch_with_prompt(name)?;
            }
        }
        Err(e) if can_force_remove_worktree(&e) => {
            let force = dialoguer::Confirm::new()
                .with_prompt(format!(
                    "Worktree '{path}' has uncommitted changes. Force delete?"
                ))
                .default(false)
                .interact()
                .map_err(|e| e.to_string())?;
            if force {
                crate::git::remove_worktree(path, true, None)?;
                eprintln!("Force removed worktree: {path}");

                let also_delete_branch = dialoguer::Confirm::new()
                    .with_prompt(format!("Also delete branch '{name}'?"))
                    .default(false)
                    .interact()
                    .map_err(|e| e.to_string())?;
                if also_delete_branch {
                    delete_branch_with_prompt(name)?;
                }
            }
        }
        Err(e) => eprintln!("Error removing worktree: {e}"),
    }
    Ok(())
}

fn handle_delete(item: &Item) -> Result<(), String> {
    match item {
        Item::Branch { name, is_current } => {
            if *is_current {
                eprintln!(
                    "Cannot delete the current branch '{name}'. Switch to another branch first."
                );
                return Ok(());
            }
            delete_branch_with_prompt(name)?;
        }
        Item::Worktree { name, path } => remove_worktree_with_prompt(name, path)?,
        Item::Header => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' && chars.peek() == Some(&'[') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch == 'm' {
                        break;
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    #[test]
    fn build_display_list_has_branch_header() {
        let dl = build_display_list(vec![], vec![]);
        assert!(
            dl.display
                .iter()
                .any(|s| strip_ansi(s).contains("Branches")),
            "expected a Branches header"
        );
    }

    #[test]
    fn build_display_list_no_worktree_header_when_empty() {
        let dl = build_display_list(vec![("main".into(), true)], vec![]);
        assert!(
            !dl.display
                .iter()
                .any(|s| strip_ansi(s).contains("Worktrees")),
            "should not show Worktrees header when there are no worktrees"
        );
    }

    #[test]
    fn build_display_list_marks_current_branch_with_asterisk() {
        let branches = vec![("main".into(), true), ("feat".into(), false)];
        let dl = build_display_list(branches, vec![]);
        assert!(
            dl.display
                .iter()
                .any(|s| strip_ansi(s).trim_start() == "* main"),
            "current branch should be marked with *"
        );
        assert!(
            dl.display
                .iter()
                .any(|s| strip_ansi(s).trim_start() == "feat"),
            "non-current branch should not have *"
        );
    }

    #[test]
    fn build_display_list_includes_worktree_section() {
        let wts = vec![Worktree {
            name: "feat".into(),
            path: "/p/feat".into(),
        }];
        let dl = build_display_list(vec![("main".into(), true)], wts);
        assert!(dl
            .display
            .iter()
            .any(|s| strip_ansi(s).contains("Worktrees")));
        assert!(dl
            .display
            .iter()
            .any(|s| s.contains("feat") && s.contains("/p/feat")));
    }

    #[test]
    fn build_display_list_items_and_display_same_length() {
        let branches = vec![("main".into(), true), ("feat".into(), false)];
        let wts = vec![Worktree {
            name: "wt".into(),
            path: "/p".into(),
        }];
        let dl = build_display_list(branches, wts);
        assert_eq!(dl.items.len(), dl.display.len());
    }

    #[test]
    fn build_display_list_worktree_columns_aligned() {
        let wts = vec![
            Worktree {
                name: "short".into(),
                path: "/a".into(),
            },
            Worktree {
                name: "a-longer-name".into(),
                path: "/b".into(),
            },
        ];
        let dl = build_display_list(vec![], wts);
        let positions: Vec<usize> = dl
            .display
            .iter()
            .filter(|s| s.contains("/a") || s.contains("/b"))
            .map(|s| s.find('/').unwrap_or(0))
            .collect();
        assert!(
            positions.windows(2).all(|w| w[0] == w[1]),
            "worktree paths should start at the same column: {positions:?}"
        );
    }

    #[test]
    fn detect_force_removable_worktree_error() {
        assert!(can_force_remove_worktree(
            "fatal: 'C:\\\\repo\\\\wt' contains modified or untracked files, use --force to delete it"
        ));
        assert!(!can_force_remove_worktree(
            "fatal: cannot remove the current working tree"
        ));
    }
}
