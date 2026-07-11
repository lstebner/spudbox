import { describe, expect, it } from "vitest";
import { sortAlbums } from "./sort";
import type { AlbumRow } from "$lib/types";

function makeAlbum(overrides: Partial<AlbumRow> & Pick<AlbumRow, "id" | "title" | "album_artist">): AlbumRow {
  return {
    album_artist_id: 1,
    year: null,
    art_path: null,
    rating: null,
    date_added: null,
    is_new: false,
    ...overrides,
  };
}

const radiohead = makeAlbum({ id: 1, title: "OK Computer", album_artist: "Radiohead", date_added: 3000 });
const beatles = makeAlbum({ id: 2, title: "Abbey Road", album_artist: "The Beatles", date_added: 1000 });
const beethoven = makeAlbum({ id: 3, title: "Symphony No. 5", album_artist: "Beethoven", date_added: 2000 });
const beatlesWhite = makeAlbum({ id: 4, title: "The White Album", album_artist: "The Beatles", date_added: 500 });

const albums = [radiohead, beatles, beethoven, beatlesWhite];

describe("sortAlbums — date_added", () => {
  it("sorts newest first", () => {
    const result = sortAlbums(albums, "date_added");
    expect(result.map((a) => a.id)).toEqual([1, 3, 2, 4]);
  });

  it("treats null date_added as 0 (sorts last)", () => {
    const noDate = makeAlbum({ id: 5, title: "Timeless", album_artist: "Unknown", date_added: null });
    const result = sortAlbums([radiohead, noDate], "date_added");
    expect(result[0].id).toBe(radiohead.id);
    expect(result[1].id).toBe(noDate.id);
  });

  it("does not mutate the input array", () => {
    const input = [beatles, radiohead];
    sortAlbums(input, "date_added");
    expect(input[0]).toBe(beatles);
    expect(input[1]).toBe(radiohead);
  });
});

describe("sortAlbums — artist_name", () => {
  it("sorts by artist name alphabetically", () => {
    const result = sortAlbums(albums, "artist_name");
    expect(result[0].album_artist).toBe("Beethoven");
    expect(result[1].album_artist).toBe("Radiohead");
  });

  it("breaks ties within the same artist by album title", () => {
    const result = sortAlbums(albums, "artist_name");
    const beatlesAlbums = result.filter((a) => a.album_artist === "The Beatles");
    expect(beatlesAlbums[0].title).toBe("Abbey Road");
    expect(beatlesAlbums[1].title).toBe("The White Album");
  });

  it("does not mutate the input array", () => {
    const input = [radiohead, beatles];
    sortAlbums(input, "artist_name");
    expect(input[0]).toBe(radiohead);
  });
});

describe("sortAlbums — album_name", () => {
  it("sorts by album title alphabetically", () => {
    const result = sortAlbums(albums, "album_name");
    expect(result[0].title).toBe("Abbey Road");
    expect(result[1].title).toBe("OK Computer");
    expect(result[2].title).toBe("Symphony No. 5");
    expect(result[3].title).toBe("The White Album");
  });

  it("does not mutate the input array", () => {
    const input = [beatles, radiohead];
    sortAlbums(input, "album_name");
    expect(input[0]).toBe(beatles);
  });
});
