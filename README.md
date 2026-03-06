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

### Search Tracks

Another really useful command is the ability to search songs through the terminal!

To try it out, run the following command:
```bash
lastui search "name of song"
```

For example, if I want to search for [Feel Special by TWICE](https://www.youtube.com/watch?v=3ymwOvzhwHs) then I would run the following command:

```bash
lastui search "Feel Special"  
```

This will give me the following output:

```bash
Feel Special - TWICE (692006 listeners)
Feel Special - HavinMotion (635 listeners)
Feel Special - Funguypiano (1080 listeners)
Feel Special - Hikaru Station (1072 listeners)
Feel Special - TWICE (트와이스) (1259 listeners)
Feel Special - 트와이스 (699 listeners)
Feel Special - Pianella Piano (366 listeners)
feel special - demon gummies (308 listeners)
Feel Special - TWICE(트와이스) (900 listeners)
Feel Special - Kairo Mouse (323 listeners)
```

Go ahead! Try it out :D
