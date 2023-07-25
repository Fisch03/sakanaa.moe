const music = document.getElementById('MusicList');

window.addEventListener('load',fetchlastfm);

function fetchlastfm() {
  fetch('/api/lastfm')
  .then(res => res.json())
  .then(data => {
    console.log(data);

    let content = '';
    data.topTracks.forEach(song => {
      content += `
      ${song.link!=undefined?`<a href=${song.link}>`:""}
        <div class="song">
          <div style="width: 7rem; height: 7rem; border-radius: 1rem;margin-bottom: 10px; margin-right: 15px;" class="shadow-box"><img style="width: 7rem; border-radius: 1rem;" class="colorfilter" src=${song.cover!=undefined?song.cover:"assets/coverdefault.png"} alt="Album Art" id="AlbumArt"></div>  
          <div>
            <h3 style="margin-bottom: 0px; margin-top: 0px">${song.name}</h3>
            <p style="margin-top: 0; margin-bottom: 8px">${song.artist}</p>
            <p style="margin-top: 0; margin-bottom: 0px">${song.playcount}x in the past month</p>
          </div>
        </div>
        ${song.link!=undefined?`</a>`:""}
      `
    });
    music.innerHTML = content;
  })
}