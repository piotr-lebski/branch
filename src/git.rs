use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct Worktree {
    pub name: String,
    pub path: String,
}

fn run_git_in(args: &[&str], dir: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn list_local_branches(cwd: Option<&Path>) -> Result<Vec<(String, bool)>, String> {
    let output = run_git_in(&["branch", "--format=%(HEAD)%(refname:short)"], cwd)?;
    let mut branches = Vec::new();
    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let is_current = line.starts_with('*');
        let name = if is_current {
            line[1..].to_string()
        } else {
            line.trim_start().to_string()
        };
        branches.push((name, is_current));
    }
    Ok(branches)
}

pub fn list_worktrees(cwd: Option<&Path>) -> Result<Vec<Worktree>, String> {
    let output = run_git_in(&["worktree", "list", "--porcelain"], cwd)?;
    let mut worktrees = Vec::new();
    let blocks: Vec<&str> = output.split("\n\n").collect();
    for block in blocks.iter().skip(1) {
        let mut path = None;
        let mut branch_name = None;
        for line in block.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                path = Some(p.to_string());
            } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
                branch_name = Some(b.to_string());
            }
        }
        if let (Some(p), Some(b)) = (path, branch_name) {
            worktrees.push(Worktree { name: b, path: p });
        }
    }
    Ok(worktrees)
}

pub fn list_remote_branches(cwd: Option<&Path>) -> Result<Vec<String>, String> {
    let output = run_git_in(&["branch", "-r", "--format=%(refname:short)"], cwd)?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.ends_with("/HEAD"))
        .map(str::to_string)
        .collect())
}

pub fn checkout_branch(name: &str, cwd: Option<&Path>) -> Result<(), String> {
    run_git_in(&["checkout", name], cwd).map(|_| ())
}

pub fn delete_branch(name: &str, force: bool, cwd: Option<&Path>) -> Result<(), String> {
    let flag = if force { "-D" } else { "-d" };
    run_git_in(&["branch", flag, name], cwd).map(|_| ())
}

pub fn remove_worktree(path: &str, cwd: Option<&Path>) -> Result<(), String> {
    run_git_in(&["worktree", "remove", path], cwd).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn setup_repo() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "t@t.com"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(p)
            .output()
            .unwrap();
        std::fs::write(p.join("f"), "x").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(p)
            .output()
            .unwrap();
        let path = p.to_path_buf();
        (dir, path)
    }

    #[test]
    fn list_local_branches_returns_main_as_current() {
        let (_dir, path) = setup_repo();
        let branches = list_local_branches(Some(&path)).unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].0, "main");
        assert!(branches[0].1, "main should be current");
    }

    #[test]
    fn list_local_branches_marks_only_one_current() {
        let (_dir, path) = setup_repo();
        Command::new("git")
            .args(["branch", "feature"])
            .current_dir(&path)
            .output()
            .unwrap();
        let branches = list_local_branches(Some(&path)).unwrap();
        assert_eq!(branches.len(), 2);
        let current_count = branches.iter().filter(|(_, c)| *c).count();
        assert_eq!(current_count, 1);
    }

    #[test]
    fn list_local_branches_non_current_branch_not_marked() {
        let (_dir, path) = setup_repo();
        Command::new("git")
            .args(["branch", "feature"])
            .current_dir(&path)
            .output()
            .unwrap();
        let branches = list_local_branches(Some(&path)).unwrap();
        let feature = branches.iter().find(|(name, _)| name == "feature").unwrap();
        assert!(!feature.1, "feature should not be current");
    }

    #[test]
    fn list_worktrees_empty_when_no_worktrees() {
        let (_dir, path) = setup_repo();
        let wts = list_worktrees(Some(&path)).unwrap();
        assert!(wts.is_empty(), "main worktree must not appear in the list");
    }

    #[test]
    fn list_worktrees_returns_added_worktree() {
        let (_dir, path) = setup_repo();
        let wt_dir = tempfile::tempdir().unwrap();
        Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                "feat",
                wt_dir.path().to_str().unwrap(),
            ])
            .current_dir(&path)
            .output()
            .unwrap();
        let wts = list_worktrees(Some(&path)).unwrap();
        assert_eq!(wts.len(), 1);
        assert_eq!(wts[0].name, "feat");
        assert_eq!(wts[0].path, wt_dir.path().to_str().unwrap());
    }

    #[test]
    fn list_remote_branches_excludes_head_and_returns_branches() {
        // Create an upstream repo with a feature branch
        let (upstream_dir, upstream_path) = setup_repo();
        Command::new("git")
            .args(["branch", "feature"])
            .current_dir(&upstream_path)
            .output()
            .unwrap();

        // Clone it so we get a remote tracking setup
        let clone_dir = tempfile::tempdir().unwrap();
        Command::new("git")
            .args([
                "clone",
                upstream_path.to_str().unwrap(),
                clone_dir.path().to_str().unwrap(),
            ])
            .output()
            .unwrap();
        // Disable gpgsign in the clone too
        Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(clone_dir.path())
            .output()
            .unwrap();

        let remotes = list_remote_branches(Some(clone_dir.path())).unwrap();
        // Should contain "origin/feature" and "origin/main" but NOT "origin/HEAD"
        assert!(
            remotes.iter().any(|r| r == "origin/main"),
            "expected origin/main: {remotes:?}"
        );
        assert!(
            remotes.iter().any(|r| r == "origin/feature"),
            "expected origin/feature: {remotes:?}"
        );
        assert!(
            !remotes.iter().any(|r| r.ends_with("/HEAD")),
            "must not include HEAD: {remotes:?}"
        );

        drop(upstream_dir);
    }

    #[test]
    fn checkout_branch_switches_branch() {
        let (_dir, path) = setup_repo();
        Command::new("git")
            .args(["branch", "feature"])
            .current_dir(&path)
            .output()
            .unwrap();
        checkout_branch("feature", Some(&path)).unwrap();
        let branches = list_local_branches(Some(&path)).unwrap();
        let feature = branches.iter().find(|(n, _)| n == "feature").unwrap();
        assert!(feature.1, "feature should now be current after checkout");
    }

    #[test]
    fn checkout_branch_errors_on_nonexistent() {
        let (_dir, path) = setup_repo();
        let result = checkout_branch("ghost", Some(&path));
        assert!(
            result.is_err(),
            "checking out nonexistent branch should fail"
        );
    }

    #[test]
    fn delete_branch_removes_merged_branch() {
        let (_dir, path) = setup_repo();
        Command::new("git")
            .args(["branch", "todelete"])
            .current_dir(&path)
            .output()
            .unwrap();
        delete_branch("todelete", false, Some(&path)).unwrap();
        let branches = list_local_branches(Some(&path)).unwrap();
        assert!(!branches.iter().any(|(n, _)| n == "todelete"));
    }

    #[test]
    fn delete_branch_fails_on_unmerged_without_force() {
        let (_dir, path) = setup_repo();
        Command::new("git")
            .args(["checkout", "-b", "unmerged"])
            .current_dir(&path)
            .output()
            .unwrap();
        std::fs::write(path.join("new_file"), "data").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "unmerged commit"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&path)
            .output()
            .unwrap();
        let result = delete_branch("unmerged", false, Some(&path));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("not fully merged"),
            "error should mention 'not fully merged': {err}"
        );
    }

    #[test]
    fn delete_branch_force_removes_unmerged_branch() {
        let (_dir, path) = setup_repo();
        Command::new("git")
            .args(["checkout", "-b", "unmerged"])
            .current_dir(&path)
            .output()
            .unwrap();
        std::fs::write(path.join("new_file"), "data").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "unmerged commit"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&path)
            .output()
            .unwrap();
        delete_branch("unmerged", true, Some(&path)).unwrap();
        let branches = list_local_branches(Some(&path)).unwrap();
        assert!(!branches.iter().any(|(n, _)| n == "unmerged"));
    }

    #[test]
    fn remove_worktree_removes_worktree() {
        let (_dir, path) = setup_repo();
        let wt_dir = tempfile::tempdir().unwrap();
        let wt_path = wt_dir.path().to_str().unwrap().to_string();
        Command::new("git")
            .args(["worktree", "add", "-b", "feat", &wt_path])
            .current_dir(&path)
            .output()
            .unwrap();
        remove_worktree(&wt_path, Some(&path)).unwrap();
        let wts = list_worktrees(Some(&path)).unwrap();
        assert!(wts.is_empty());
    }
}
