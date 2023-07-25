import dotenv from "dotenv";
dotenv.config();

import fetch from "node-fetch";

import TrackDB from "./trackdb.js";
import { AlbumData, Playable, TrackData } from "./tracktypes.js";

interface LastFMTopTracksResponse {
  toptracks: {
    track: {
      name: string;
      playcount: number;
      mbid: string;
      artist: {
        name: string;
        mbid: string;
      };
    }[];
  };
}

interface LastFMTopAlbumsResponse {
  topalbums: {
    album: {
      name: string;
      playcount: number;
      mbid: string;
      artist: {
        name: string;
        mbid: string;
      };
    }[];
  };
}

interface DisplayableTrack extends TrackData { cover?: string; }
interface DisplayableAlbum extends AlbumData { }
type Displayables = DisplayableTrack | DisplayableAlbum;


export default class LastFM {
  TrackDB = new TrackDB();
  
  topTracks: DisplayableTrack[] = [];
  topAlbums: DisplayableAlbum[] = [];

  public init() {
    setInterval(() => {
      this.update();
    }, 1000 * 60 * 60);

    this.update();
  }

  public update() {
    console.log('updating LastFM data...');

    this.doLastFMRequest<LastFMTopTracksResponse>('user.gettoptracks', `user=Fisch03&period=1month`)
    .then(data => {
      this.topTracks = [];
      data.toptracks.track.forEach(async (track) => {
        let trackData: TrackData = new TrackData(track.name, track.artist.name);
        if(track.mbid && track.mbid != "") trackData.mbid = track.mbid;
        
        trackData = await this.TrackDB.fillData<TrackData>(trackData);

        trackData.playcount = track.playcount;

        let displayedTrack: DisplayableTrack = trackData;
        if(trackData.albumId) {
          let album = await this.TrackDB.getAlbumFromID(trackData.albumId);
          if(album) displayedTrack.cover = album.cover;
        }

        this.topTracks.push(displayedTrack);

        this.sortByPlaycount<DisplayableTrack>(this.topTracks);
      });
    });

    this.doLastFMRequest<LastFMTopAlbumsResponse>('user.gettopalbums', `user=Fisch03&period=1month`)
    .then(data => {
      this.topAlbums = [];
      data.topalbums.album.forEach(async (album) => {
        let albumData: AlbumData = new AlbumData(album.name, album.artist.name);
        if(album.mbid && album.mbid != "") albumData.mbid = album.mbid;
        
        albumData = await this.TrackDB.fillData<AlbumData>(albumData);

        albumData.playcount = album.playcount;

        let displayedAlbum: DisplayableAlbum = albumData;
        this.topAlbums.push(displayedAlbum);

        this.sortByPlaycount<DisplayableAlbum>(this.topAlbums);
      });
    });
  }

  get Tops() {
    return {
      topTracks: this.topTracks,
      topAlbums: this.topAlbums,
    }
  }

  private sortByPlaycount<T extends Displayables>(displayables: T[]) {
    displayables.sort((a, b) => {
      if(!a.playcount) return 1;
      if(!b.playcount) return -1;
      return Number(b.playcount) - Number(a.playcount);
    });
  }

  private doLastFMRequest<T>(method: string, params: string) {
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

  
}