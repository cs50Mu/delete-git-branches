use chrono::{DateTime, FixedOffset, TimeZone};
use git2::{BranchType, Oid, Repository};
use std::fmt::write;
use std::io::{self, Bytes, Stdin, Stdout};
use std::io::{Read, Write};
use std::string::FromUtf8Error;

// shortcut for Result<T, Error>
// 第二个 Result 要全路径指定，否则会形成递归定义
type Result<T, E = Error> = std::result::Result<T, E>;

fn main() {
    // 放在闭包里，可以确保最后的 disable_raw_mode 一定会执行
    let result = (|| -> Result<_> {
        let repo = Repository::open_from_env()?;

        crossterm::terminal::enable_raw_mode()?;

        let mut stdout = io::stdout();
        let mut stdin = io::stdin().bytes();

        let mut branches = get_branches(&repo)?;
        if branches.is_empty() {
            write!(stdout, "found no branches(ignore master / main branch)\r\n")?;
        }

        // mutable loop
        for br in &mut branches {
            match get_user_action(&mut stdout, &mut stdin, br)? {
                BranchAction::Keep => {
                    write!(stdout, "keep this br\r\n")?;
                } //
                BranchAction::Delete => {
                    write!(stdout, "delete this br\r\n")?;
                    br.delete()?;
                } //
                BranchAction::Quit => {
                    write!(stdout, "\r\n")?;
                    break;
                }
                _ => {}
            }
        }
        Ok(())
    })();

    crossterm::terminal::disable_raw_mode().ok();

    match result {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    }
}

fn get_user_action(
    stdout: &mut Stdout,
    stdin: &mut Bytes<Stdin>,
    br: &Branch,
) -> Result<BranchAction> {
    // d: delete
    // k: keep
    // s: show
    // ?: help
    write!(
        stdout,
        "'{}' ({}) last commit at {} (d/k/s/?) > ",
        // 一个类型若实现了 Display trait，则会自动实现 ToString trait
        // ref: https://doc.rust-lang.org/nightly/alloc/string/trait.ToString.html
        br.name,
        &br.commit_id.to_string()[..10],
        br.commit_time
    )?;
    stdout.flush()?;
    let byte = match stdin.next() {
        Some(byte) => byte?,
        None => return get_user_action(stdout, stdin, br),
    };
    let c = char::from(byte);
    write!(stdout, "{}\r\n", c)?;
    stdout.flush()?;
    let action: BranchAction = c.try_into()?;
    Ok(match action {
        BranchAction::Help => {
            // write!(stdout, "\r\n")?;
            write!(stdout, "Here are what the commands mean:\r\n")?;
            write!(stdout, "k - Keep the branch\r\n")?;
            write!(stdout, "d - Delete the branch\r\n")?;
            write!(stdout, "s - Show the branch\r\n")?;
            write!(stdout, "q - Quit\r\n")?;
            write!(stdout, "? - Print this help\r\n")?;
            stdout.flush()?;
            return get_user_action(stdout, stdin, br);
        } //
        action => action,
    })
}

fn get_branches(repo: &Repository) -> Result<Vec<Branch>> {
    let mut branches = repo
        .branches(Some(BranchType::Local))?
        .map(|br| -> Result<Branch> {
            let (branch, _) = br?;
            let name = String::from_utf8(branch.name_bytes()?.to_vec())?;
            // stdout.write_all(name)?;
            // write!(stdout, "\n")?;
            let commit = branch.get().peel_to_commit()?;
            let commit_id = commit.id();
            let commit_time = commit.time();
            let commit_time = FixedOffset::east(commit_time.offset_minutes() * 60)
                .timestamp(commit_time.seconds(), 0);
            Ok(Branch {
                commit_time,
                commit_id,
                name,
                branch,
            })
        })
        .filter(|branch| {
            if let Ok(branch) = branch {
                branch.name != "master" && branch.name != "main"
            } else {
                true
            }
        })
        .collect::<Result<Vec<_>>>()?;

    branches.sort_unstable_by_key(|br| br.commit_time);

    Ok(branches)
}

// #[derive(Debug)]
struct Branch<'repo> {
    commit_time: DateTime<FixedOffset>,
    commit_id: Oid,
    name: String,
    branch: git2::Branch<'repo>,
}

impl Branch<'_> {
    fn delete(&mut self) -> Result<()> {
        Ok(self.branch.delete()?)
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    // Errors may use error(transparent) to forward the source and
    // Display methods straight through to an underlying error without adding an additional message.
    #[error(transparent)]
    CrosstermError(#[from] crossterm::ErrorKind),
    // // #[error(transparent)]
    // IoError(#[from] io::Error),
    #[error(transparent)]
    GitError(#[from] git2::Error),

    #[error(transparent)]
    Utf8Error(#[from] FromUtf8Error),

    #[error("invalid input, don't know what '{0}' means")]
    InvalidInput(char),
}

enum BranchAction {
    Keep,
    Delete,
    Help,
    Show,
    Quit,
}

impl TryFrom<char> for BranchAction {
    type Error = Error;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'k' => Ok(BranchAction::Keep),
            'd' => Ok(BranchAction::Delete),
            '?' => Ok(BranchAction::Help),
            's' => Ok(BranchAction::Show),
            'q' => Ok(BranchAction::Quit),
            _ => Err(Error::InvalidInput(value)),
        }
    }
}
