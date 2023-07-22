import gulp from 'gulp';
const { series,parallel, dest,src } = gulp;

import { deleteAsync } from "del";
import concat from 'gulp-concat';
import cp from 'child_process';
const exec = cp.exec;

import minify from 'gulp-minify';
import htmlmin from 'gulp-htmlmin';
import htmlReplace from 'gulp-html-replace';
import cleanCSS from 'gulp-clean-css';
import imagemin from 'gulp-imagemin';

import ts from 'gulp-typescript';

import { createProxyMiddleware } from 'http-proxy-middleware';
import bs from 'browser-sync';
import nodemon from 'nodemon';
const browserSync = bs.create();



/* Server Tasks */
const serversrc = 'src/server/';
const serverout = 'dist/server/';
let server = ts.createProject('tsconfig.json');

function cleanServer() {
  return deleteAsync(['dist/server']);
}

function makeFolders() {
  return src('*.*', {read: false})
    .pipe(dest('db'))
}

function transpileServer() {
  return server.src().pipe(server()).js.pipe(dest(serverout));
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
function compressImages() {
  return src(pagesrc+'/assets/**/*')
    .pipe(imagemin())
    .pipe(dest(pageout + '/assets'));
}

gulp.task('buildPage', series(cleanPage, copyFiles, parallel(minifyJs, minifyHtml, minifyCss, compressImages)));

/* Server Tasks */
function runServer(cb) {
  exec('node dist/server/server.js', (err, stdout, stderr) => {
    console.log(stdout);
    console.log(stderr);
    cb(err);
  });
}

function devRunServer(cb) {
  let stream = nodemon({
    watch: ["src/server/server.ts", "src/server/*"],
    exec: 'ts-node-esm src/server/server.ts',
  });

  stream.on('restart', () => {
    console.log('retranspiling server...');
  });
  
  return stream;
}

function devRunBrowserSync() {
  let apiProxy = createProxyMiddleware('/api', {target: 'http://localhost:3000'});

  browserSync.init({
    server: {
      baseDir: pagesrc,
      middleware: [apiProxy],
    },
    open: false,
    port: 3001,
  });

  gulp.watch(pagesrc+'**/*', (cb) => {
    browserSync.reload();
    cb();
  });
}

/* Main Tasks */
gulp.task('clean', parallel(()=>deleteAsync(['dist']), ()=>deleteAsync(['db'])));
gulp.task('build', parallel('buildServer','buildPage'))
gulp.task('run', series(makeFolders, 'build', runServer))

gulp.task('dev',series(makeFolders, parallel(devRunServer,devRunBrowserSync)));

export default series('build');