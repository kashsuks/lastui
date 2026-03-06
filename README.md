# Lastui

A command line tool to acces the Last.fm API and its available commands

## Installation

Lastui can be installed through [crates](https://crates.io) through the following [link](https://crates.io/crates/lastui)

Or by running the following command in your terminal:

```bash
cargo install lastui
```

## Usage

Lastui takes advantage of the public Last.fm API in order to allow
you to make actions through the terminal.

### Get Started

In order to perform any command in Lastui, you first need to autheticate yourself. You need the following:

- Last.fm account (sign up [here](https://www.last.fm/join))
- Last.fm developer API key (don't worry they're free and you can get them from [here](https://www.last.fm/api/account/create))

Once you have the needed information, go to your terminal and type in the following command

```bash
lastui
```

You should be prompted to enter your Last.fm API key and then your username.

Once everything works, you are good to go!

### Recent Tracks
Use this command if you would like to see your most recent tracks.

_Requires the following_: 
- Last.fm API key
- Last.fm Username (your username)

__Usage__:
Use the following command after authorizing yourself

```bash
lastui recent-tracks
```

Expect an output like the following:

```
> Allergy - i-dle (now playing)
    Die with a Smile - Lady Gaga
    HOT - LE SSERAFIM
    ONE SPARK (Instrumental) - TWICE
    Darl+ing - Seventeen
    Rich Man - aespa
    Smart (Instrumental) - LE SSERAFIM
    songs that remind me of you - natalie jinju
    Blue Flame - LE SSERAFIM
    STYLE - Hearts2Hearts
    ABCD (Extended Version) - NAYEON
```
