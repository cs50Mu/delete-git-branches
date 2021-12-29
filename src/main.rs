use chrono::{DateTime, FixedOffset, TimeZone};
use git2::{BranchType, Oid, Repository};
use std::io;
use std::io::{Read, Write};
use std::string::FromUtf8Error;

fn main() -> Result<(), Error> {
    let mut stdout = io::stdout();

    let repo = Repository::open_from_env()?;

    for br in get_branches(&repo)? {
        println!("{:?}", br);
    }

    Ok(())
}

// shortcut for Result<T, Error>
// 第二个 Result 要全路径指定，否则会形成递归定义
type Result<T, E = Error> = std::result::Result<T, E>;

fn get_branches(repo: &Repository) -> Result<Vec<Branch>> {
    // let mut branches = vec![];
    // for br in repo.branches(Some(BranchType::Local))? {
    //     let (branch, _) = br?;
    //     let name = String::from_utf8(branch.name_bytes()?.to_vec())?;
    //     // stdout.write_all(name)?;
    //     // write!(stdout, "\n")?;
    //     let commit = branch.get().peel_to_commit()?;
    //     let commit_id = commit.id();
    //     let commit_time = commit.time();
    //     println!("sign: {}", commit_time.sign());
    //     let commit_time = FixedOffset::east(commit_time.offset_minutes() * 60).timestamp(commit_time.seconds(), 0);
    //     // write!(stdout, "{:?}\n", datetime)?;
    //     branches.push(Branch{
    //         commit_time,
    //         commit_id,
    //         name,
    //     });
    // }

    // Ok(branches)

    repo.branches(Some(BranchType::Local))?
        .map(|br| -> Result<Branch> {
            let (branch, _) = br?;
            let name = String::from_utf8(branch.name_bytes()?.to_vec())?;
            // stdout.write_all(name)?;
            // write!(stdout, "\n")?;
            let commit = branch.get().peel_to_commit()?;
            let commit_id = commit.id();
            let commit_time = commit.time();
            // println!("sign: {}", commit_time.sign());
            let commit_time = FixedOffset::east(commit_time.offset_minutes() * 60)
                .timestamp(commit_time.seconds(), 0);
            Ok(Branch {
                commit_time,
                commit_id,
                name,
            })
        })
        .collect()
}

#[derive(Debug)]
struct Branch {
    commit_time: DateTime<FixedOffset>,
    commit_id: Oid,
    name: String,
}

// fn main() -> Result<(), Error>{
//     crossterm::terminal::enable_raw_mode()?;

//     let mut stdout = io::stdout();
//     let mut stdin = io::stdin().bytes();

//     loop {
//         write!(stdout, "type something> ")?;
//         stdout.flush()?;
//         // let byte = stdin.next().unwrap();
//         let byte = match stdin.next() {
//             Some(byte) => byte?,
//             None => break,
//         };
//         let c = char::from(byte);
//         if  c == 'q' {
//             break;
//         }
//         write!(stdout, "You pressed '{}'\n\r", c)?;

//         // let byte = byte.unwrap();
//         // stdout.write_all(&[byte]).unwrap();
//         stdout.flush()?;
//     }

//     crossterm::terminal::disable_raw_mode()?;
//     Ok(())
// }

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
}
