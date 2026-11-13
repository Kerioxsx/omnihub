#!/bin/bash
# Vesper's hit sounds, synthesized from formulas with ffmpeg (original, no samples):
#   bash client/sounds/make_hit_sounds.sh
set -e
D="$(dirname "$0")/../src/main/resources/assets/vesper/sounds/hit"
mkdir -p "$D"
mk(){ ffmpeg -hide_banner -loglevel error -y -f lavfi -i "aevalsrc=$2:s=44100:d=$3" -ac 1 -c:a libvorbis -q:a 4 "$D/$1.ogg"; }
mk pop "0.9*sin(2*PI*(900-6000*t)*t)*exp(-t*38)" 0.12
mk ding "0.6*(sin(2*PI*1760*t)+0.5*sin(2*PI*2640*t)+0.25*sin(2*PI*3520*t))*exp(-t*9)" 0.6
mk thud "0.9*sin(2*PI*(140-300*t)*t)*exp(-t*22)+0.25*(random(0)-0.5)*exp(-t*60)" 0.25
mk click "0.8*(random(0)-0.5)*exp(-t*220)+0.5*sin(2*PI*3000*t)*exp(-t*120)" 0.06
mk bass "0.95*sin(2*PI*(80-60*t)*t)*exp(-t*10)" 0.4
mk crit "0.55*(sin(2*PI*1320*t)+sin(2*PI*1980*t)+0.6*sin(2*PI*2970*t))*exp(-t*7)*(1+0.3*sin(2*PI*30*t))" 0.7
