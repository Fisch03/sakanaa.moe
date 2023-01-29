import cv2, os
import numpy as np

outStr = 'const palettes = [\n'

for file in os.listdir('palettes'):
  img = cv2.imread(os.path.join('palettes', file))
  fg = img[0,0]
  bg = img[0,1]

  fg[0] = min(254, fg[0])
  fg[1] = min(254, fg[1])
  fg[2] = min(254, fg[2])

  if((0.2126*fg[2] + 0.7152*fg[0] + 0.0722*fg[1]) > (0.2126*bg[2] + 0.7152*bg[0] + 0.0722*bg[1])):
    fg, bg = bg, fg

  outStr += f'  {{n: "{file[0:-4].replace("-", " ").replace("!","")}",fg: {{ r: {fg[2]}, g: {fg[1]}, b: {fg[0]} }}, bg: {{ r: {bg[2]}, g: {bg[1]}, b: {bg[0]} }}}},\n'

outStr += '];'

with open('js/!palettes.js', 'w') as f:
  f.write(outStr)
