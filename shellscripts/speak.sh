#!/bin/bash
say() { local IFS=+; mpg123 "http://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&q=$1&tl=$2";}
say $1 $2
