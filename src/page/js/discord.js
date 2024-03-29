const discordDiv = document.getElementById('DiscordStats');

window.addEventListener('load', updateDiscord);
setInterval(updateDiscord, 10000);

function updateDiscord() {
  fetch('https://api.lanyard.rest/v1/users/431374517462499328')
    .then(res => res.json())
    .then(data => {
      data = data.data;
      let avatar = `https://cdn.discordapp.com/avatars/${data.discord_user.id}/${data.discord_user.avatar}.png`;
      document.getElementById('DiscordAvatar').src = avatar;
      document.getElementById('DiscordName').innerText = `${data.discord_user.username}`
      if(data.discord_status == 'offline') {
        document.getElementById('DiscordActivity').innerHTML = '<div style="width: 100%; height: 90%; display:flex; justify-content: center; align-items:center;"><h2>Offline</h2></div>';
      } else if(data.listening_to_spotify) {
        document.getElementById('DiscordActivity').innerHTML = `
        <a href="https://open.spotify.com/track/${data.spotify.track_id}">
          <div style="width: 10rem; height: 10rem; border-radius: 1rem; margin-bottom: 10px; margin-left: 5px;" class="shadow-box"><img style="width: 10rem; height: 10rem; border-radius: 1rem;" class="colorfilter" src=${data.spotify.album_art_url} alt="Album Art" id="AlbumArt"></div>
          <div>
            <h3><img style="display: inline-block; width: 20px; margin-right:4px;" class="bounce colorfilter" src="/assets/notespin.png">im listening to</h3>
            <h2 style="max-width: 20rem; margin-bottom: 5px">${data.spotify.song}</h2>
            <h4 style="max-width: 20rem; margin-top: 0; margin-bottom: 0">from ${data.spotify.artist}</h4>
            <h4 style="max-width: 20rem; margin-top: 0">on ${data.spotify.album}</h4>
          </div>
        </a>
        `
      } else if(data.activities.length > 0) {
        let musicactivity = data.activities.find(activity => activity.name == 'MusicBee');

        if(musicactivity) {
            let artist = musicactivity.state.split(' on ');
            artist.pop();
            artist = artist.join(' on ');
            let song = musicactivity.details;
            let image = musicactivity.assets.large_image.startsWith('mp:external/')
              ? `https://media.discordapp.net/external/${musicactivity.assets.large_image.replace("mp:external/", "")}` 
              : `https://cdn.discordapp.com/app-assets/${musicactivity.application_id}/${musicactivity.assets.large_image}.webp`;
            document.getElementById('DiscordActivity').innerHTML = `
              <div style="width: 10rem; height: 10rem; border-radius: 1rem; margin-bottom: 10px; margin-left: 5px;" class="shadow-box"><img style="width: 10rem; border-radius: 1rem;" class="colorfilter" src=${image} alt="Album Art" id="AlbumArt"></div>
              <div>
                <h3><img style="display: inline-block; width: 20px; margin-right:4px;" class="bounce colorfilter" src="/assets/notespin.png">im listening to</h3>
                <h2 style="max-width: 20rem; margin-bottom: 5px">${song}</h2>
                <h4 style="max-width: 20rem; margin-top: 0">${artist}</h4>
                
              </div>
            </a>
            `
        } else {
          let activity = data.activities[0];
          let image = activity.assets.large_image.startsWith('mp:external/')
            ? `https://media.discordapp.net/external/${activity.assets.large_image.replace("mp:external/", "")}` 
            : `https://cdn.discordapp.com/app-assets/${activity.application_id}/${activity.assets.large_image}.webp`;
          document.getElementById('DiscordActivity').innerHTML = `
          <img style="width: 10rem; height:10rem; border-radius: 1rem; margin-bottom: 10px; margin-left: 5px;" class="shadow-box colorfilter" src=${image} alt="Album Art" id="AlbumArt">
          <div>
            <h3 style="margin-bottom: 10px;">im playing</h3>
            <h2 style="max-width: 20rem; margin-bottom: 5px; margin-top: 0">${activity.name}</h2>
            <h4 style="max-width: 20rem; margin-bottom: 5px; margin-top: 0">${activity.details}</h4>
            <h4 style="max-width: 20rem; margin-top: 0">${activity.state}</h4>
          </div>
          `
        }
      } else {
        document.getElementById('DiscordActivity').innerHTML = '<div style="width: 100%; height: 90%; display:flex; justify-content: center; align-items:center;"><h3>looks like i\'m not doing anything right now...</h3></div>';
      }
    }) 
}