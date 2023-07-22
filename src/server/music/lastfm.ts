import dotenv from "dotenv";
dotenv.config();

import fetch from "node-fetch";

import TrackDB from "./trackdb.js";
import { TrackData } from "./tracktypes.js";

/*
import SpotifyWebApi from "spotify-web-api-node";

let spotify = new SpotifyWebApi({
  clientId: process.env.SPOTIFYID,
  clientSecret: process.env.SPOTIFYKEY
});
spotify.clientCredentialsGrant().then(
  function(data) {
    spotify.setAccessToken(data.body.access_token);
  },
  function(err) {
    console.log('Something went wrong when retrieving an access token', err);
  }
)
*/

interface LastFMTopTracksResponse {
  toptracks: {
    track: {
      name: string;
      playcount: number;
      artist: {
        name: string;
      };
    }[];
  };
}

/*
interface SpotifySearchResponse {
  tracks: {
    items: {
      album: {
        external_urls: {
          spotify: string;
        };
        images: {
          url: string;
        }[];
      };
    }[];
  };
}
*/

export default class LastFM {
  TrackDB = new TrackDB();
  
  topTracks: TrackData[] = [];

  public init() {
    setInterval(() => {
      this.update();
    }, 1000 * 60 * 60);

    this.update();
  }

  public update() {
    this.doLastFMRequest<LastFMTopTracksResponse>('user.gettoptracks', `user=Fisch03&period=1month`)
    .then(data => {
      this.topTracks = [];
      data.toptracks.track.forEach(async (track) => {
        let trackData: TrackData = {
          name: track.name,
          artist: track.artist.name,
        };
        
        trackData = await this.TrackDB.fillTrackData(trackData);

        trackData.playcount = track.playcount;

        this.topTracks.push(trackData);
      });
    });
  }

  /*
  updateCovers() {
    this.topTracks.forEach(track => {
      if(LastFM.coverCache.has(track)) {
        track.cover = LastFM.coverCache.get(track);
      } else {
        this.doSpotifyRequest(track)
        .then(data => {
          if(data.tracks.items.length > 0) {
            track.cover = data.tracks.items[0].album.images[0].url;
            track.link = data.tracks.items[0].album.external_urls.spotify;
            LastFM.coverCache.set(track, data.tracks.items[0].album.images[0].url);
          }
        })
      }
    });
  }
  */

  doLastFMRequest<T>(method: string, params: string) {
    return new Promise<T>((resolve, reject) => {
      fetch('https://ws.audioscrobbler.com/2.0/', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
          'User-Agent': 'sakanaa.moe',
        },
        body: `api_key=${process.env.LASTFMKEY}&format=json&method=${method}&${params}`
      })
      .then(response => {
        if(response.status !== 200) throw new Error(response.statusText);
        resolve(response.json() as Promise<T>);
      });
    });
  }	

  /*
  doSpotifyRequest(track: Track) {
    return new Promise<SpotifySearchResponse>((resolve, reject) => {
      spotify.searchTracks(`artist:${track.artist} ${track.name}`)
      .then(data => {
        resolve(data.body as SpotifySearchResponse);
      });
    });
  }
  */

  getTracks(): TrackData[] {
    return this.topTracks;
  }
}