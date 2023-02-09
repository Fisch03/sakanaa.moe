import dotenv from "dotenv";
dotenv.config();

import fetch from "node-fetch";
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


interface TrackData {
  name: string;
  artist: string;
}
interface Track extends TrackData {
  cover?: string;
}
interface TopTrack extends Track {
  playcount: number;
  link?: string;
}

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

export default class LastFM{
  static coverCache: Map<TrackData, string> = new Map();
  responseCache: LastFMTopTracksResponse | null = null;
  
  topTracks: TopTrack[] = [];

  public init() {
    setInterval(() => {
      this.update();
    }, 1000 * 60 * 60);

    this.update();
  }

  public update() {
    this.doLastFMRequest<LastFMTopTracksResponse>('user.gettoptracks', `user=Fisch03&period=7days`)
    .then(data => {
      this.topTracks = [];
      this.responseCache = data as LastFMTopTracksResponse;
      this.responseCache.toptracks.track.forEach(track => {
        this.topTracks.push({
          name: track.name,
          artist: track.artist.name,
          cover: undefined,
          playcount: track.playcount,
          link: undefined
        });
      });
      this.updateCovers();
    });
  }

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
        /*
        this.doLastFMRequest<LastFMTrack>('track.getInfo', `artist=${track.artist}&track=${track.name}`)
        .then(data => {
          if(!data.track.album || data.track.album.image.length < 4 || data.track.album.image[3]["#text"] == '') {
            track.cover = undefined;
          } else {
            track.cover = data.track.album.image[3]["#text"];
            LastFM.coverCache.set(track, data.track.album.image[3]["#text"]);
            return 
          }
        })
        */
      }
    });
  }

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

  doSpotifyRequest(track: Track) {
    return new Promise<SpotifySearchResponse>((resolve, reject) => {
      spotify.searchTracks(`artist:${track.artist} ${track.name}`)
      .then(data => {
        resolve(data.body as SpotifySearchResponse);
      });
    });
  }

  getTracks(): Track[] {
    return this.topTracks;
  }
}