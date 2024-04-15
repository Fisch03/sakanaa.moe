const palettes = [
  {n: "ys flowers and asbestos 1x",fg: { r: 198, g: 43, b: 105 }, bg: { r: 237, g: 244, b: 255 }},
  {n: "1bit monitor glow 1x",fg: { r: 34, g: 35, b: 35 }, bg: { r: 240, g: 246, b: 240 }},
  {n: "casio basic 1x",fg: { r: 0, g: 0, b: 0 }, bg: { r: 131, g: 176, b: 126 }},
  {n: "default",fg: { r: 16, g: 16, b: 16 }, bg: { r: 255, g: 255, b: 255 }},
  {n: "gato roboto chewed gum 1x",fg: { r: 41, g: 22, b: 29 }, bg: { r: 250, g: 148, b: 149 }},
  {n: "knockia3310 1x",fg: { r: 33, g: 44, b: 40 }, bg: { r: 114, g: 164, b: 136 }},
  {n: "nokia 3310 1x",fg: { r: 67, g: 82, b: 61 }, bg: { r: 199, g: 240, b: 216 }},
  {n: "paperback 2 1x",fg: { r: 56, g: 43, b: 38 }, bg: { r: 184, g: 194, b: 185 }},
  {n: "peachy keen 1x",fg: { r: 36, g: 34, b: 52 }, bg: { r: 250, g: 202, b: 184 }},
  {n: "runes spells 1x",fg: { r: 0, g: 4, b: 18 }, bg: { r: 217, g: 185, b: 130 }},
  {n: "ys funky jam 1x",fg: { r: 146, g: 2, b: 68 }, bg: { r: 254, g: 194, b: 140 }},
  {n: "ys neutral green 1x",fg: { r: 0, g: 76, b: 61 }, bg: { r: 255, g: 234, b: 249 }},
];

document.addEventListener('DOMContentLoaded', async () => {
    let c = document.createElement('canvas');
    let ctx = c.getContext('2d', { willReadFrequently: true, alpha: true });
    let backgrounds = document.querySelectorAll('.background');

    let preparedImages = await prepareImages(c, ctx);
    console.log(preparedImages);
    let pointer = 0;
    let bgcolor = palettes[pointer].bg;
    let fgcolor = palettes[pointer].fg;
    backgrounds.forEach(bg => bg.style.backgroundColor = `rgb(${bgcolor.r}, ${bgcolor.g}, ${bgcolor.b})`);
    applyColors(c, ctx, fgcolor, bgcolor, preparedImages, backgrounds);

    document.getElementById("ColorBtn").addEventListener('click', (e) => {
      pointer = Math.random() * palettes.length | 0;

      fgcolor = palettes[pointer].fg;
      bgcolor = palettes[pointer].bg;
      console.log(palettes[pointer].n);

      applyColors(c, ctx, fgcolor, bgcolor, preparedImages, backgrounds);
    });

    document.body.addEventListener('htmx:afterSwap', (e) => {
      applyElementColors(c, ctx, fgcolor, bgcolor, e.detail.target, preparedImages.ditherimg);
    })
});

async function prepareImages(c, ctx) {
  let promises = [];
  let result = {
    imgs: [],
    ditherimg: {},
    cursorimg: {},
    cursorhoverimg: {}
  }

  document.querySelectorAll('.paletteimg').forEach(htmlimg => {
    promises.push(new Promise((resolve) => {
      const img = new Image();
      img.onload = () => {
        let bits = getBits(c, ctx, img)
        result.imgs.push({
          e: htmlimg,
          b: bits,
          w: img.width,
          h: img.height
        })
        resolve();
      };
      img.src = htmlimg.src;
    }));
  });

  promises.push(new Promise((resolve) => {
    getDitherImage(c, ctx, 1, document.querySelector('.onex').style.backgroundImage)
      .then((ditherimg) => { 
          result.ditherimg[1] = ditherimg;
          return getDitherImage(c, ctx, 2, document.querySelector('.twox').style.backgroundImage) 
      })
      .then((ditherimg) => { 
          result.ditherimg[2] = ditherimg;
          resolve() 
      });
  }));

  promises.push(new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      let bits = getBits(c, ctx, img)
      result.cursorimg = {
        b: bits,
        w: img.width,
        h: img.height
      };
      resolve();
    };
    img.src = 'assets/cursor.png';
  }));
  promises.push(new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      let bits = getBits(c, ctx, img)
      result.cursorhoverimg = {
        b: bits,
        w: img.width,
        h: img.height
      };
      resolve();
    };
    img.src = 'assets/cursorhover.png';
  }));

  await Promise.all(promises);
  return result;
}

function getDitherImage(c, ctx, factor, imgn) {
  let src = imgn.substring(5, imgn.length - 2);
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      let bits = getBits(c, ctx, img)
      let ditherimg = {
        b: bits,
        w: img.width,
        h: img.height
      };
      resolve(ditherimg);
    }
    img.src = src;
  });
}

function getBits(c, ctx, img) {
  c.width = img.width;
  c.height = img.height;
  ctx.drawImage(img, 0, 0);

  let data = ctx.getImageData(0, 0, img.width, img.height);

  let bits = [];
  for (let i = 0; i < data.data.length; i += 4) {
    if (data.data[i + 3] == 0) {
      bits.push(0);
    } else if (data.data[i] >= 200 && data.data[i + 1] >= 200 && data.data[i + 2] >= 200) {
      bits.push(1);
    } else {
      bits.push(2);
    }
  }
  return (bits);
}

function applyElementColors(c, ctx, fgcolor, bgcolor, e, ditherimg) {
  if (ditherimg[1] == undefined || ditherimg[2] == undefined) return;

  e.querySelectorAll('.onex').forEach(e => { e.style.backgroundImage = `url('${replaceColors(c, ctx, fgcolor, bgcolor, ditherimg[1])}')` });
  e.querySelectorAll('.twox').forEach(e => { e.style.backgroundImage = `url('${replaceColors(c, ctx, fgcolor, bgcolor, ditherimg[2])}')` });
}

function applyColors(c, ctx, fgcolor, bgcolor, preparedImages, backgrounds) {
  backgrounds.forEach(bg => bg.style.backgroundColor = `rgb(${bgcolor.r}, ${bgcolor.g}, ${bgcolor.b})`);

  document.querySelector(':root').style.setProperty('--fg-color', `rgb(${fgcolor.r}, ${fgcolor.g}, ${fgcolor.b})`);
  document.querySelector(':root').style.setProperty('--bg-color', `rgb(${bgcolor.r}, ${bgcolor.g}, ${bgcolor.b})`);

  document.querySelector(':root').style.setProperty('--cursor', `url('${replaceColors(c, ctx, fgcolor, bgcolor, preparedImages.cursorimg)}')`);
  document.querySelector(':root').style.setProperty('--cursor-hover', `url('${replaceColors(c, ctx, fgcolor, bgcolor, preparedImages.cursorhoverimg)}')`);

  applyElementColors(c, ctx, fgcolor, bgcolor, document, preparedImages.ditherimg);

  let avg = { r: Math.sqrt((fgcolor.r ** 2 + bgcolor.r ** 2) / 2), g: Math.sqrt((fgcolor.g ** 2 + bgcolor.g ** 2) / 2), b: Math.sqrt((fgcolor.b ** 2 + bgcolor.b ** 2) / 2) };
  let brightness = (0.2126 * avg.r + 0.7152 * avg.g + 0.0722 * avg.b) / 255;
  document.getElementById('colorfiltermatrix').setAttribute('values', `1 0 0 0 ${1 - (avg.r / 255)}  0 1 0 0 ${1 - (avg.g / 255)}  0 0 1 0 ${1 - (avg.b / 255)}  0 0 0 1 0`);
  document.querySelectorAll('.colorfilterbrightness').forEach(e => { e.setAttribute('slope', 1.8 - brightness) });

  preparedImages.imgs.forEach(img => img.e.src = replaceColors(c, ctx, fgcolor, bgcolor, img));
}

function replaceColors(c, ctx, fgcolor, bgcolor, img) {
  let data = new Uint8ClampedArray(4 * img.b.length)

  for (let i = 0; i < img.b.length; i++) {
    switch (img.b[i]) {
      case 2:
        data[i * 4] = fgcolor.r;
        data[i * 4 + 1] = fgcolor.g;
        data[i * 4 + 2] = fgcolor.b;
        data[i * 4 + 3] = 255;
        break;
      case 1:
        data[i * 4] = bgcolor.r;
        data[i * 4 + 1] = bgcolor.g;
        data[i * 4 + 2] = bgcolor.b;
        data[i * 4 + 3] = 255;
        break;
      case 0:
        data[i * 4] = 0;
        data[i * 4 + 1] = 0;
        data[i * 4 + 2] = 0;
        data[i * 4 + 3] = 0;
    }
  }
  c.width = img.w;
  c.height = img.h;
  ctx.putImageData(new ImageData(data, img.w, img.h), 0, 0);

  return c.toDataURL();
}
