#!/bin/bash
play() { local IFS=+; mpg123 -q -a hw:2,0 "$1";}
play "$1" 
