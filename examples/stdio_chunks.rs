use std::io::{self, Read};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

const CHUNK_SIZE: usize = 1024;

fn main()
{
    let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    // Spawn reader thread
    let reader_handle = thread::spawn(move || {
        reader_thread(tx);
    });

    // Spawn processor thread
    let processor_handle = thread::spawn(move || {
        processor_thread(rx);
    });

    // Wait for both threads to complet
    match reader_handle.join()
    {
        Ok(_) => todo!(),
        Err(err) => eprintln!("Reader thread panicked: {:?}", err),
    }

    match processor_handle.join()
    {
        Ok(_) => todo!(),
        Err(err) => eprintln!("Processor thread panicked: {:?}", err),
    }
}

fn reader_thread(tx: Sender<Vec<u8>>)
{
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf = [0u8; CHUNK_SIZE];

    loop
    {
        match handle.read(&mut buf)
        {
            Ok(0) => break, // EOF
            Ok(n) =>
            {
                if tx.send(buf[.. n].to_vec()).is_err()
                {
                    break; // Receiver dropped
                }
            }
            Err(e) =>
            {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }
}

fn processor_thread(rx: Receiver<Vec<u8>>)
{
    let mut leftover = [0u8; 4];
    let mut leftover_len = 0;

    for chunk in rx
    {
        // Combine leftover bytes with new chunk
        let mut combined = Vec::with_capacity(leftover_len + chunk.len());
        combined.extend_from_slice(&leftover[.. leftover_len]);
        combined.extend_from_slice(&chunk);

        let mut i = 0;
        while i < combined.len()
        {
            // Determine UTF-8 character byte length
            let char_len = utf8_char_width(combined[i]);

            if char_len == 0
            {
                // Invalid UTF-8 start byte, skip it
                eprintln!("Invalid UTF-8 byte: {:02x}", combined[i]);
                i += 1;
                continue;
            }

            // Check if we have enough bytes for complete character
            if i + char_len > combined.len()
            {
                // Incomplete character at end, save to leftover
                leftover_len = combined.len() - i;
                leftover[.. leftover_len].copy_from_slice(&combined[i ..]);
                break;
            }

            // Extract and validate the character
            let char_bytes = &combined[i .. i + char_len];
            match std::str::from_utf8(char_bytes)
            {
                Ok(s) =>
                {
                    for c in s.chars()
                    {
                        print!("{}", c);
                    }
                }
                Err(_) =>
                {
                    eprintln!("Invalid UTF-8 sequence at position {}", i);
                }
            }

            i += char_len;
        }

        // If we processed everything, clear leftover
        if i >= combined.len()
        {
            leftover_len = 0;
        }

        let resultant_str = unsafe { String::from_utf8_lossy(&combined[.. i]) };
        println!("{}", resultant_str);
    }

    // Handle any remaining leftover bytes at end of stream
    if leftover_len > 0
    {
        eprintln!(
            "Incomplete UTF-8 sequence at end of input ({} bytes)",
            leftover_len
        );
    }
}

// Determine the expected length of a UTF-8 character from its first byte
fn utf8_char_width(first_byte: u8) -> usize
{
    match first_byte
    {
        0b0000_0000 ..= 0b0111_1111 => 1, // 0xxxxxxx
        0b1100_0000 ..= 0b1101_1111 => 2, // 110xxxxx
        0b1110_0000 ..= 0b1110_1111 => 3, // 1110xxxx
        0b1111_0000 ..= 0b1111_0111 => 4, // 11110xxx
        _ => 0,                           // Invalid start byte
    }
}
