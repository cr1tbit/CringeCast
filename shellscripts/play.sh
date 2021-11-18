#!/bin/bash
play() { local IFS=+; mpg123 "$1";}
play "$1" 
