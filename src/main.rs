use std::io;
use std::io::{Read, Write};
use git2::Repository;

fn main() -> Result<(), Error> {
    let repo = Repository::open_from_env()?;

    Ok(())
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
}