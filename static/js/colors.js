const c = document.createElement('canvas');
const ctx = c.getContext('2d', { willReadFrequently: true, alpha: true });
const backgrounds = document.querySelectorAll('.background');

let imgs = [];
let ditherimg = {};
let cursorimg = {};
let cursorhoverimg = {};

pointer = 0;

let fgcolor = palettes[pointer].fg;
let bgcolor = palettes[pointer].bg;
backgrounds.forEach(bg => bg.style.backgroundColor = `rgb(${bgcolor.r}, ${bgcolor.g}, ${bgcolor.b})`);

function prepareImages() {
  promises = [];
  document.querySelectorAll('.paletteimg').forEach(htmlimg => {
    promises.push(new Promise((resolve) => {
      const img = new Image();
      img.onload = () => {
        let bits = getBits(img)
        imgs.push({
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
    getDitherImage(1, document.querySelector('.onex').style.backgroundImage)
      .then(() => { return getDitherImage(2, document.querySelector('.twox').style.backgroundImage) })
      .then(() => { resolve() });
  }));

  promises.push(new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      let bits = getBits(img)
      cursorimg = {
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
      let bits = getBits(img)
      cursorhoverimg = {
        b: bits,
        w: img.width,
        h: img.height
      };
      resolve();
    };
    img.src = 'assets/cursorhover.png';
  }));

  return Promise.all(promises);
}

function getDitherImage(factor, imgn) {
  return new Promise((resolve) => {
    let src = imgn.substring(5, imgn.length - 2);
    const img = new Image();
    img.onload = () => {
      let bits = getBits(img)
      ditherimg[factor] = {
        b: bits,
        w: img.width,
        h: img.height
      };
      resolve();
    }
    img.src = src;
  });
}

function getBits(img) {
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

window.addEventListener('load', () => {
  prepareImages()
    .then(() => {
      applyColors();
    })

  document.getElementById("ColorBtn").addEventListener('click', (e) => {
    pointer = Math.random() * palettes.length | 0;

    fgcolor = palettes[pointer].fg;
    bgcolor = palettes[pointer].bg;
    console.log(palettes[pointer].n);

    applyColors();
  });
});


document.body.addEventListener('htmx:afterSwap', (e) => {
  applyElementColors(e.detail.target);
})

function applyElementColors(e) {
  if (ditherimg[1] == undefined || ditherimg[2] == undefined) return;

  e.querySelectorAll('.onex').forEach(e => { e.style.backgroundImage = `url('${replaceColors(ditherimg[1])}')` });
  e.querySelectorAll('.twox').forEach(e => { e.style.backgroundImage = `url('${replaceColors(ditherimg[2])}')` });
}

function applyColors() {
  backgrounds.forEach(bg => bg.style.backgroundColor = `rgb(${bgcolor.r}, ${bgcolor.g}, ${bgcolor.b})`);

  document.querySelector(':root').style.setProperty('--fg-color', `rgb(${fgcolor.r}, ${fgcolor.g}, ${fgcolor.b})`);
  document.querySelector(':root').style.setProperty('--bg-color', `rgb(${bgcolor.r}, ${bgcolor.g}, ${bgcolor.b})`);

  document.querySelector(':root').style.setProperty('--cursor', `url('${replaceColors(cursorimg)}')`);
  document.querySelector(':root').style.setProperty('--cursor-hover', `url('${replaceColors(cursorhoverimg)}')`);

  applyElementColors(document);

  let avg = { r: Math.sqrt((fgcolor.r ** 2 + bgcolor.r ** 2) / 2), g: Math.sqrt((fgcolor.g ** 2 + bgcolor.g ** 2) / 2), b: Math.sqrt((fgcolor.b ** 2 + bgcolor.b ** 2) / 2) };
  let brightness = (0.2126 * avg.r + 0.7152 * avg.g + 0.0722 * avg.b) / 255;
  document.getElementById('colorfiltermatrix').setAttribute('values', `1 0 0 0 ${1 - (avg.r / 255)}  0 1 0 0 ${1 - (avg.g / 255)}  0 0 1 0 ${1 - (avg.b / 255)}  0 0 0 1 0`);
  document.querySelectorAll('.colorfilterbrightness').forEach(e => { e.setAttribute('slope', 1.8 - brightness) });

  imgs.forEach(img => img.e.src = replaceColors(img));
}

function replaceColors(img) {
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
