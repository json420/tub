Installing Tub
==============

On account of being so kickass, Tub is written in Rust, which you can install
from here:

    https://rustup.rs/

Then build and install Tub like this:

    $ cargo install --path .

The `tub` CLI tool is our current toy DVCS app built on top of the Bathtub DB
object store.

For available commands, run `tub` with no arguments:

    $ tub

For example, initialize a Tub repository like this:

    $ tub init

Oh and definetly hash a big file (like a GB or more) with `tub hash <filename>`
and then do the same with `git hash-object <filename>`:

    $ time tub hash /some/big/file
    $ time git hash-object /some/big/file

WTF is Tub?  A database, version control software, or what?
===========================================================

The goal of Tub is to improve the operational efficiency of complex human
systems (companies, governments, nonprofits, etc).  Go faster, solve harder
problems, make the world a better place... that kinda stuff.

Tub is made for *ALL* of your data.  From source code to raw video files to
structured data to ML training sets.  The way Tub can improve operational
efficiency is by unifying all your data into a single, coherent system.

The Tub vision goes something like this... take a design like Git, generalize it
and squeeze as much performance out as possible... and then use Tub for
absolutely freakin' everything!

ALL YOUR DATUM ARE BELONG TO TUB!


Other Rust version control projects
===================================

https://github.com/Byron/gitoxide/

https://pijul.org/

