# CringeCast - cast cringe through your speakers, as rendered on your big Bro's cloud

This project allows you to make sound through the host device's speakers, either playing some .mp3 files, or just speech-synthesized phrases, using `google translate`'s unsecured API endpoint lmao.

API (GET requests):
* `/` - frontend may be found here
* `/*` - say anything, try to guess the language
* `/*?l=xx` - say anything, but force specific language
* `/say/` - say anything, in english
* `/mow/` - say anything, in polish
* `/play/` - play a predefined file from `audio_files` directory
* `/vol/` - set volume, range 0-100

## Project history, or "why is it so shitty.txt"

When I learned that google translate's speech synthesis endpoint is not secured by any means, I just had to do it. You can send a query with any text, and they just return you an mp3 file.

The project emerged as fun little script that allowed people in Hackerspace to anonymously speak using their browser address bar. The script would run on a NanoPi neo hidden in an old radio player. I would call it "The Cringebox".

At some point, I decided to allow access to the device from the outside - now people could remotely troll^Wtell people stuff physically apparent at hackerspace.

THEN, someone decided to integrate this utility into our discord bot, so that when someone asks about people at hackerspace on discord, the cringeCast would announce that to everyone in physical space.

At some point, our doorbell stopped working. So we just glued an QR code with cringebox's address, that would just say "ding dong" when scanned. It literally worked.

It was really happening - cringebox started becoming our core infrastructure.

Back then, there was no frontend yet, people still used browser's bar (or wget scripts) to say stuff - I decided my child needs frontend. So I just copied it from my older project, and it kinda works now, but is super messy.

I also added volume bar at some point, when you run it on your PC, you can change your volue from your browser, lmao.

But the trolling functionality wasn't sophisticated enough yet - we needed a capability to play mp3 files, I added it with rather ease, but someone wanted a way to upload their own files too. And so it happend, my friend added that, creating a first real PR to my own project \*_\*

Keep in mind, the project synthesizes **anything** that gets requested - meaning that sometimes you may hear you device saying `robots.txt`, `favicon.ico` (this one is detected as spanish, which is even funnier), or random phrases containing "PHP-something", which are just webcrawlers looking for unsecured wordpress websites.