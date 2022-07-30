# PubliChat

PubliChat is a semi-private chatting application.
Chats are encrypted with their title as a key.
Every chat is accessible to anyone, provided they know the chat's title.
The title is never sent to the server, so the server can't decrypt the chat contents.
This way, the server does not need to be trusted.

Server written in Rust. FLOSS!

## Example uses
- Chat securely and privately by picking a secure title (like a strong password)
- Make a private note for yourself by picking a secure title and not sharing it
- Discuss topics in 'public' chats with insecure titles (eg. `Baking`, `Fishing` or `Chess`)
- Discuss webpages with no comments section (set the title to page's url)

## How to use
#### Web version
- Go to [publi.chat](https://publi.chat)
- Enter a chat title on the top to fetch messages start reading
- Enter a username and message on the bottom to send something

#### Server
- Clone the repository with `git clone git@github.com:GrishaVar/publichat.git`
- Open directory with `cd publichat`
- Launch server with `cargo r --release --bin server [socket_addr] data_directory/`
    - `socket_addr` should be an (ip or domain) with a port
    - `data_directory/` is where all chat data will be stored

#### TUI
- Clone the repository with `git clone git@github.com:GrishaVar/publichat.git`
- Open directory with `cd publichat`
- Launch client with `cargo r --release socket_addr chat_title username`
    - `socket_addr` should be an (ip or domain) with a port

## Visual explainer
![Diagram of software structure](/misc/plan.png)

## Titles
A _chat title_ is used to encrypt the chat's contents.
Do not share a chat's title with someone unless you want them to read the chat!
The title's hash is the _chat id_. Only the chat id is sent to the server.
Therefore, the server doesn't know the title and thus cannot decrypt and read the chat.

## Usernames
To verify the authorship of each message, users must choose a _username_.
This username is **not** public and should be chosen as a strong password would be.
Do not share your secret username with anyone!

Usernames are used to generate a public-key system.
Each message is signed, clients automatically verify the signatures of each incoming message.
Public keys are used to identify message authors to other users.
A malicious user can display the same ID publicly, but they can not sign their messages with it!

## Tech
- Signatures are done with [Ed19255](https://en.wikipedia.org/wiki/EdDSA#Ed25519)
- Chats are encrypted with [AES](https://en.wikipedia.org/wiki/Advanced_Encryption_Standard)
- Title and Username hashing is done with [Argon2](https://en.wikipedia.org/wiki/Argon2)
- Other hashing is done with [SHA-3](https://en.wikipedia.org/wiki/SHA-3)
