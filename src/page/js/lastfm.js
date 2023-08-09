const music = document.getElementById('MusicListContent');

let data = {
  "topTracks": [],
  "topArtists": [],
  "topAlbums": []
}

window.addEventListener('load',() => {
  fetch('/api/lastfm')
  .then(res => res.json())
  .then(d => {
    data = {
      ...data,
      ...d
    }
    console.log(data);
    showlastfm();
  });
});

function showlastfm(type = 'Album') {
  let content = '';
  data[`top${type}s`].forEach(playable => {
    content += `
    ${playable.link!=undefined?`<a href=${playable.link}>`:""}
      <div class="playable">
        <div style="width: 7rem; height: 7rem; border-radius: 1rem;margin-bottom: 10px; margin-right: 15px;" class="shadow-box"><img style="width: 7rem; border-radius: 1rem;" class="colorfilter" src=${playable.cover!=undefined?playable.cover:"assets/coverdefault.png"} alt="Album Art" id="AlbumArt"></div>  
        <div>
          <h3 style="margin-bottom: 0px; margin-top: 0px">${playable.name}</h3>
          ${playable.artistName?`<p style="margin-top: 0; margin-bottom: 8px">${playable.artistName}</p>`:""}
          <p style="margin-top: 0; margin-bottom: 0px">${playable.playcount}x in the past month</p>
        </div>
      </div>
      ${playable.link!=undefined?`</a>`:""}
    `
  });
  music.innerHTML = content;
}