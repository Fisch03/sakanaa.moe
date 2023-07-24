export enum PlayableType {
  Track = 'track',
  Album = 'album',
}

export abstract class Playable {
  type?: PlayableType;

  id?: number; // Internal ID in the database
  mbid?: string;   

  name: string;

  playcount?: number;

  constructor(name: string) {
    this.name = name;
  }
}

export class TrackData extends Playable {
  type = PlayableType.Track;

  artist: string;

  albumId?: number;
  link?: string;

  constructor(name: string, artist: string) {
    super(name);
    this.artist = artist;
  }
}

export class AlbumData extends Playable {
  type = PlayableType.Album;

  artist: string;

  cover?: string;

  constructor(name: string, artist: string) {
    super(name);
    this.artist = artist;
  }
}
