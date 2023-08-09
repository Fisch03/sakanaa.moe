import Database from 'better-sqlite3';

import MusicBrainz from './musicbrainz.js';

import { Playable, PlayableType, TrackData, AlbumData, ArtistData } from "./tracktypes.js";

type DBPlayable <T extends Playable> = T | undefined;

interface QueueItem<T extends Playable> extends Playable {
  retryCount?: number;
  resolve?: (result: T) => void;
  reject?: (reason: any) => void;
}

export default class TrackDB {
  db = new Database('db/cache.db');
  apiQueue: QueueItem<any>[] = [];
  processedAmt = 0;

  constructor() {
    this.db.pragma('journal_mode = WAL');

    this.db.exec(`CREATE TABLE IF NOT EXISTS ${PlayableType.Album}s (
      id INTEGER PRIMARY KEY,
      mbid TEXT,

      name TEXT,
      artistId INTEGER,

      cover TEXT
    )`);

    this.db.exec(`CREATE TABLE IF NOT EXISTS ${PlayableType.Track}s (
      id INTEGER PRIMARY KEY, 
      mbid TEXT, 

      name TEXT, 
      artistId INTEGER,

      albumId INTEGER,
      FOREIGN KEY(albumId) REFERENCES ${PlayableType.Album}s(id)
    )`);

    this.db.exec(`CREATE TABLE IF NOT EXISTS ${PlayableType.Artist}s (
      id INTEGER PRIMARY KEY,
      mbid TEXT,

      name TEXT,
      image TEXT
    )`);
  }

  async fillData<T extends Playable>(playable: T, immediate: boolean = false) : Promise<T> {
    let res: DBPlayable<T>;

    res = this.fillFromDB<T>(playable);
    if(!res) res = await this.fetchNew<T>(playable, immediate);

    if(!res) return playable;
    else     return res;
  }

  async fetchNew<T extends Playable>(playable: T, immediate: boolean = false) : Promise<T> {
    let queueItem = playable as QueueItem<T>;
    
    return new Promise((resolve, reject) => {
      queueItem.retryCount = 0;

      queueItem.resolve = (newItem: T | number) => {
        let rowid: number | bigint;
        if(typeof newItem == 'number') rowid = newItem;
        else rowid = this.insertPlayable<T>(newItem);

        let result = this.db.prepare(`SELECT * FROM ${playable.type}s WHERE id = ?`).get(rowid) as T;
        resolve(result);
      };

      queueItem.reject = (reason: string) => {
        if(!queueItem.retryCount) queueItem.retryCount = 0;

        if(queueItem.retryCount < 3) {
          queueItem.retryCount++;
          console.log(`Failed to process ${queueItem.type}: ${reason} Retrying...`);
          this.addToQueue<T>(queueItem);
          return;
        } else {
          reject(`Failed to insert ${queueItem.type} into database: ${reason}`);
        }
      }

      this.addToQueue<T>(queueItem, immediate);
    });
  }

  /* --- API Queue Functions --- */
  private addToQueue<T extends Playable>(queueItem: QueueItem<T>, immediate: boolean = false) {
    if(immediate) this.apiQueue.unshift(queueItem);
    else this.apiQueue.push(queueItem);
    if(this.processedAmt == 0) this.processQueue();
  }

  private async processQueue() {
    let queueItem = this.apiQueue[0];
    if(!queueItem) return;

    this.processedAmt++;
    this.apiQueue.shift();

    let hasStartedQueue = false;


    console.log(`Processing queue | total: ${this.apiQueue.length} | current: ${queueItem.type} - ${queueItem.name}`);

    let res: Playable | number | undefined;
  
    switch(queueItem.type) {
      case PlayableType.Track: {
          let track = queueItem as TrackData;

          let artist: DBPlayable<ArtistData>;
          if(track.artistId) artist = this.getArtistFromID(track.artistId);
          if(!artist && track.artistHelperField) artist = this.getArtistFromName(track.artistHelperField);
          if(!artist && track.artistHelperField) {
            console.log(`    Track is missing artist, adding artist "${track.artistHelperField}" to queue...`);
            setTimeout(() => this.processQueue(), 1200); // Timeout is 1 second min + some some safety margin
            hasStartedQueue = true;
            artist = await this.fillData<ArtistData>(new ArtistData(track.artistHelperField), true);
          }

          if(!artist) {
            console.log(`    Failed to find artist for track "${track.name}"`)
            queueItem.reject?.(`Failed to find artist for track "${track.name}"`);
            break;
          }

          res = await MusicBrainz.searchTrack(track, artist, this, (album: AlbumData) => {
            if(album.mbid) {
              let albumByMBID = this.getAlbumFromMBID(album.mbid);
              if(albumByMBID) return albumByMBID.id;
            }

            console.log(`    Inserting new album "${album.name}" into database...`);

            return Number(this.insertPlayable<AlbumData>(album));
          });
        }
        break;
      
      case PlayableType.Album: {
          let album = queueItem as AlbumData;

          let artist: DBPlayable<ArtistData>;
          if(album.artistId) artist = this.getArtistFromID(album.artistId);
          if(!artist && album.artistHelperField) artist = this.getArtistFromName(album.artistHelperField);
          if(!artist && album.artistHelperField) {
            console.log(`    Album is missing artist, adding artist "${album.artistHelperField}" to queue...`);
            setTimeout(() => this.processQueue(), 1200); // Timeout is 1 second min + some some safety margin
            hasStartedQueue = true;
            artist = await this.fillData<ArtistData>(new ArtistData(album.artistHelperField), true);
          }

          if(!artist) {
            console.log(`    Failed to find artist for album "${album.name}"`)
            queueItem.reject?.(`Failed to find artist for album "${album.name}"`);
            break;
          }

          res = await MusicBrainz.searchAlbum(album, artist);
          if(res?.mbid) {
            let albumByMBID = this.getAlbumFromMBID(res.mbid);
            if(albumByMBID) res = albumByMBID.id;
          }
        }
        break;

      case PlayableType.Artist: {
          let artist = queueItem as ArtistData;
          
          res = await MusicBrainz.searchArtist(artist);
          if(res?.mbid) {
            let artistByMBID = this.getArtistFromMBID(res.mbid);
            if(artistByMBID) res = artistByMBID.id;
          }
        }
        break;

    }

    if(!res) {
      if(queueItem.reject) queueItem.reject(`Failed to fetch info from API`);
    } 
    else {
      if(queueItem.resolve) queueItem.resolve(res);
    }

    this.processedAmt--;
    if(this.apiQueue.length > 0 && !hasStartedQueue) setTimeout(() => this.processQueue(), 1200); // Timeout is 1 second min + some some safety margin
    else if(!hasStartedQueue) console.log('Finished processing queue!');
  }

  /* --- Database Functions --- */
  private insertPlayable<T extends Playable>(playable: T) : number | bigint {
    switch(playable.type) {
      case 'track':
        let track = playable as unknown as TrackData;
        return this.db.prepare('INSERT INTO tracks (mbid, name, artistId, albumId) VALUES (?, ?, ?, ?)')
               .run(track.mbid, track.name, track.artistId, track.albumId).lastInsertRowid;
      case 'album':
        let album = playable as unknown as AlbumData;
        return this.db.prepare('INSERT INTO albums (mbid, name, artistId, cover) VALUES (?, ?, ?, ?)')
                .run(album.mbid, album.name, album.artistId, album.cover).lastInsertRowid;
      case 'artist':
        let artist = playable as unknown as ArtistData;
        return this.db.prepare('INSERT INTO artists (mbid, name, image) VALUES (?, ?, ?)')
                .run(artist.mbid, artist.name, artist.image).lastInsertRowid;

      default:
        throw new Error('Playable type not set or unknown');
    }
  }

  private fillFromDB<T extends Playable>(playable: T) : DBPlayable<T> {
    let res: DBPlayable<T>;

    if(playable.id) res = this.fillFromID(playable);
    if(!res)        res = this.fillFromName(playable);

    
    if(res) {
      res = {
        ...playable,
        ...res,
      }
    }
    return res;
  }

  private fillFromID<T extends Playable>(playable: T) : DBPlayable<T> {
    if(!playable.type) throw new Error('Playable type not set');

    return this.db.prepare(`SELECT * FROM ${playable.type}s WHERE id = ?`).get(playable.id) as DBPlayable<T>;
  }

  private fillFromName<T extends Playable>(playable: T) : DBPlayable<T> {
    if(!playable.type) throw new Error('Playable type not set');

    if('artist' in playable && playable.artist)
      return this.db.prepare(`SELECT * FROM ${playable.type}s WHERE name = ? AND artist = ?`).get(playable.name, playable.artist) as DBPlayable<T>;
    else
      return this.db.prepare(`SELECT * FROM ${playable.type}s WHERE name = ?`).get(playable.name) as DBPlayable<T>;
  }

  getAlbumFromID(id: number) : DBPlayable<AlbumData> {
    return this.db.prepare('SELECT * FROM albums WHERE id = ?').get(id) as DBPlayable<AlbumData>;
  }
  getAlbumFromMBID(mbid: string) : DBPlayable<AlbumData> {
    return this.db.prepare('SELECT * FROM albums WHERE mbid = ?').get(mbid) as DBPlayable<AlbumData>;
  }

  getArtistFromID(id: number) : DBPlayable<ArtistData> {
    return this.db.prepare('SELECT * FROM artists WHERE id = ?').get(id) as DBPlayable<ArtistData>;
  }
  getArtistFromMBID(mbid: string) : DBPlayable<ArtistData> {
    return this.db.prepare('SELECT * FROM artists WHERE mbid = ?').get(mbid) as DBPlayable<ArtistData>;
  }
  getArtistFromName(name: string) : DBPlayable<ArtistData> {
    return this.db.prepare('SELECT * FROM artists WHERE name = ?').get(name) as DBPlayable<ArtistData>;
  }

}