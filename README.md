# BigBroCast - cast cringe through your speakers, as rendered on your big Bro's cloud

Glued together using cyber-ducttape. Pls no judge.

Please note - I stole a random frontend from another project so it may look random but it kinda does its' job, so yeah.

Another thing - it seems mpg123 cannot output audio to an equalized alsa output I created on the cringebox - so I switched to mpg321 that seems to work fine. I'll update the repo some time.

```play() { local IFS=+; mpg321 -o alsa "$1";}```


