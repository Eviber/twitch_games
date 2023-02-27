use diacritics::remove_diacritics;
use std::sync::mpsc::channel;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};

const CHANNEL: &str = "eviber";

fn get_words() -> Vec<String> {
    let wordlist = include_str!("../wordlist");
    wordlist.split_whitespace().map(|s| s.to_string()).collect()
}

#[derive(Clone)]
struct Secret {
    word: String,
    tried: Vec<char>,
    found: Vec<char>,
    tries: usize,
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (c, check) in self
            .word
            .chars()
            .zip(remove_diacritics(&self.word).to_uppercase().chars())
        {
            if self.found.contains(&check) {
                write!(f, "{}", c)?;
            } else {
                write!(f, "-")?;
            }
        }
        if self.tried.is_empty() {
            return Ok(());
        }
        write!(f, " / ")?;
        ('A'..='Z')
            .filter(|c| self.tried.contains(c))
            .for_each(|c| write!(f, "{}", c).unwrap());
        Ok(())
    }
}

impl From<&str> for Secret {
    fn from(word: &str) -> Self {
        Secret {
            word: word.to_string(),
            tried: Vec::new(),
            found: Vec::new(),
            tries: 0,
        }
    }
}

impl Secret {
    fn generate(words: &[String]) -> Self {
        words[rand::random::<usize>() % words.len()].as_str().into()
    }

    fn guess(&mut self, mut c: char) {
        c = c.to_ascii_uppercase();
        self.tries += 1;
        if remove_diacritics(&self.word).to_uppercase().contains(c) {
            self.found.push(c);
        } else if !self.tried.contains(&c) {
            self.tried.push(c);
        } else {
            self.tries -= 1;
        }
    }

    fn is_solved(&self) -> bool {
        remove_diacritics(&self.word)
            .to_uppercase()
            .chars()
            .all(|c| self.found.contains(&c))
    }
}

#[tokio::main]
pub async fn main() {
    let words = get_words();
    // default configuration is to join chat as anonymous.
    let config = ClientConfig::default();
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);
    let (sender, receiver) = channel();

    let _join_handle = tokio::spawn(async move {
        while let Some(message) = incoming_messages.recv().await {
            if let twitch_irc::message::ServerMessage::Privmsg(msg) = message {
                sender.send(msg).unwrap();
            }
        }
    });

    client.join(CHANNEL.to_owned()).unwrap();

    let mut secret = Secret::generate(&words);
    println!("{}", secret);
    while let Ok(msg) = receiver.recv() {
        if msg.message_text.len() != 1 {
            continue;
        }
        let guess = remove_diacritics(&msg.message_text).chars().next().unwrap();
        secret.guess(guess);
        println!("{} - {}", secret, msg.sender.name);
        if secret.is_solved() {
            println!("Solved in {} tries! 🎉", secret.tries);
            secret = Secret::generate(&words);
            println!("New word!");
            println!("{}", secret);
        }
    }

    // join_handle.await.unwrap();
}
