const waifu = document.getElementById('BigWaifu');


let x = 0;
let y = 0;

let wx = 0;
let wy = 0;


window.addEventListener('mousemove', function(e) {
  wx = e.clientX/(window.innerWidth)
  wy = e.clientY/(window.innerHeight)
});

(function loop() {
  if(x != wx || y != wy) {
    x += (wx - x) / 10;
    y += (wy - y) / 10;

    console.log(x, y);

    waifu.style.transform = `translate(${x*20}px, ${y*20}px)`;
    waifu.style.zIndex = 1000;
  }
  requestAnimationFrame(loop);
})();