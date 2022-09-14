#!/bin/bash
play() { local IFS=+; mpg123 -q "$1";}
play "$1" 
