#!/bin/bash
amixer sget 'Line Out' | awk -F"[][]" '/%/ { print $2; exit }' | sed 's/%//'
