// use std::io;
// use std::time::Duration;
// use tokio::io::AsyncReadExt;
// use tokio::sync::mpsc;

// pub struct StdinReader
// {
//     receiver: mpsc::Receiver<String>,
// }

// impl StdinReader
// {
//     pub fn new() -> Self
//     {
//         let (tx, rx) = mpsc::channel(100);

//         tokio::spawn(async move {
//             let mut stdin = tokio::io::stdin();
//             let mut buffer = [0u8; 1024];

//             loop
//             {
//                 match stdin.read(&mut buffer).await
//                 {
//                     Ok(0) => break, // EOF
//                     Ok(n) =>
//                     {
//                         let text =
//                             String::from_utf8_lossy(&buffer[.. n]).to_string();
//                         if tx.send(text).await.is_err()
//                         {
//                             break; // Receiver dropped
//                         }
//                     }
//                     Err(e) =>
//                     {
//                         eprintln!("Error reading stdin: {}", e);
//                         break;
//                     }
//                 }
//             }
//         });

//         Self { receiver: rx }
//     }

//     pub fn try_read(&mut self) -> Option<String>
//     {
//         self.receiver.try_recv().ok()
//     }
// }

// impl Default for StdinReader
// {
//     fn default() -> Self
//     {
//         Self::new()
//     }
// }

// #[tokio::main]
// async fn main() -> io::Result<()>
// {
//     let mut reader = StdinReader::new();
//     println!("StdinReader started. Type something...");

//     loop
//     {
//         // Example of how to do this:
//         // tokio::time::sleep(Duration::from_millis(1000)).await;

//         while let Some(input) = reader.try_read()
//         {
//             println!("... {}", input);
//         }
//     }
// }

fn main() {}
