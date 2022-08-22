#!/bin/bash
play() { local IFS=+; mpg123 -q -a hw:2,0 "$1";}

TMPNAME=$(mktemp)
wget -q "translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&q=$1&tl=$2" -O $TMPNAME
play $TMPNAME 
