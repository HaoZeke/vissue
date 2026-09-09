A directory that is not a tracker says so instead of answering "none". The root
falls back to the working directory, and a reading verb run from somewhere else
found no projects and printed `0` with a zero exit. A caller cannot tell that
from a tracker with nothing in it, and the two mean opposite things. A guessed
root now has to hold `vissue.toml` or the prefix directory. A root named with
`--root` or `VISSUE_ROOT` is trusted whether or not it holds anything, and `man`
and `completions` still work from anywhere because they describe the program
rather than a corpus.
