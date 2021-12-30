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

        let mut deleted_branch = None;
        // mutable loop
        for br in &mut branches {
            if br.is_head {
                write!(
                    stdout,
                    "ignoring {}, because it's the current branch\r\n",
                    br.name
                )?;
                continue;
            }
            match get_user_action(&mut stdout, &mut stdin, br)? {
                BranchAction::Keep => {
                    write!(stdout, "keep this br\r\n")?;
                } //
                BranchAction::Delete => {
                    br.delete()?;
                    write!(stdout, "deleted {} ({})\r\n", br.name, br.commit_id)?;
                    deleted_branch = Some(br);
                } //
                BranchAction::Undo => {
                    // mutable reference 没有自动实现 copy，不可变reference 才会自动实现 copy
                    // 这里再加了一层引用就能编译通过了，可能是因为最外面的 reference 是不可变的了
                    if let Some(branch) = &deleted_branch {
                        // 然后后面的写法因为编译器的自动 deref，跟之前的写法一样
                        let commit = repo.find_commit(branch.commit_id)?;
                        repo.branch(&branch.name, &commit, false)?;
                    } else {
                        write!(stdout, "cannot find anything to undo!\r\n")?;
                    }
                    deleted_branch = None;
                }
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
            write!(stdout, "u - Undo the delete action\r\n")?;
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
                // 下面两个字段若换顺序则会报错：value borrowed after move
                // 因为 is_head 需要通过 borrow 来获取，而 branch 字段是 move 进去的
                is_head: branch.is_head(),
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
    is_head: bool,
    branch: git2::Branch<'repo>,
}

impl Branch<'_> {
    fn delete(&mut self) -> Result<()> {
        // Ok(self.branch.delete()?)
        // 下面这种写法貌似更优雅？
        self.branch.delete().map_err(From::from)
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
    Undo,
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
            'u' => Ok(BranchAction::Undo),
            '?' => Ok(BranchAction::Help),
            's' => Ok(BranchAction::Show),
            'q' => Ok(BranchAction::Quit),
            _ => Err(Error::InvalidInput(value)),
        }
    }
}
