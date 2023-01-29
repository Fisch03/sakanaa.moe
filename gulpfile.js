import gulp from 'gulp';
const { series,parallel, dest,src } = gulp;

import { deleteAsync } from "del";
import concat from 'gulp-concat';

import minify from 'gulp-minify';
import htmlmin from 'gulp-htmlmin';
import cleanCSS from 'gulp-clean-css';

import ts from 'gulp-typescript';
import htmlReplace from 'gulp-html-replace';

import bs from 'browser-sync';
const browserSync = bs.create();

import cp from 'child_process';
const exec = cp.exec;

/* Server Tasks */
const serversrc = 'src/server/';
const serverout = 'dist/server/';
let server = ts.createProject('tsconfig.json');

function cleanServer() {
  return deleteAsync(['dist/server']);
}

function transpileServer() {
  return server.src().pipe(server()).js.pipe(dest(serverout));
}
function transpileDevServer() {
  return server.src().pipe(server()).js.pipe(dest('src/server/'));
}

gulp.task('buildServer', series(cleanServer, transpileServer));

/* Web Page Tasks */
const pagesrc = 'src/page/';
const pageout = 'dist/page/';

function cleanPage() {
  return deleteAsync(['dist/page']);
}

function copyFiles() {
  return src([
    pagesrc+'/**/*', 
    '!'+pagesrc+'/js/**',
    '!'+pagesrc+'/js/',

    '!'+pagesrc+'/**/*.html',

    '!'+pagesrc+'/css/**',
    '!'+pagesrc+'/css/', 

    '!'+pagesrc+'palettes/**',
    '!'+pagesrc+'palettes/',

    '!'+pagesrc+'/**/*.py'
  ]).pipe(dest(pageout));
}

function minifyJs() {
  return src(pagesrc+'/js/*.js')
    .pipe(concat('script.js'))
    .pipe(minify({noSource: true}))
    .pipe(dest(pageout + '/js'));
}
function minifyHtml() {
  return src(pagesrc+'/**/*.html')
    .pipe(htmlReplace({'js': 'js/script-min.js'}))
    .pipe(htmlmin({ collapseWhitespace: true, removeComments: true }))
    .pipe(dest(pageout));
}
function minifyCss() {
  return src(pagesrc+'/css/*.css')
    .pipe(cleanCSS())
    .pipe(dest(pageout + '/css'));
}

gulp.task('buildPage', series(cleanPage, copyFiles, parallel(minifyJs, minifyHtml, minifyCss)));

/* Main Tasks */

gulp.task('clean', () => deleteAsync(['dist']));
gulp.task('build', parallel('buildServer','buildPage'))

function devServer(cb) {
  browserSync.init({
    server: {
      baseDir: pagesrc,
    },
    port: 3001,
  });

  gulp.watch(pagesrc+'**/*', (cb) => {
    browserSync.reload();
    cb();
  });

  exec('node src/server/index.js', (err, stdout, stderr) => {
    console.log(stdout);
    console.log(stderr);
    cb(err);
  });
}

gulp.task('dev',
  series( 
    transpileDevServer,
    devServer
));

export default series('build');