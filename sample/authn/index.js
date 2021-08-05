/// Waterwheel Development Proxy
/// This is a simple JavaScript proxy that authenticates users,
/// maintains sessions and proxies requests to the main Waterwheel
/// server with special headers attached. The default OPA sample 
/// policy uses the special headers to authorize requests.

const express = require('express');
const redis = require('redis');
const session = require('express-session');
const bodyParser = require("body-parser");
const morgan = require('morgan')

const RedisStore = require('connect-redis')(session);
const redisClient = redis.createClient();

const httpProxy = require('http-proxy');


const passport = require('passport')
const LocalStrategy = require('passport-local').Strategy;


// local strategy - accepts any username and password (clearly only for dev purposes!)
passport.use(new LocalStrategy(
  function(username, password, done) {
    console.log(username, 'logged in');
    return done(null, {
      username: username
    });
  }
));

passport.serializeUser(function(user, done) {
  console.log('serialise', user);
  done(null, user);
});

passport.deserializeUser(function(user, done) {
  console.log('deserialise', user);
  done(null, user);
});


const app = express();
app.use(morgan('dev'))

// store sessions in a Redis - so they survive server restarts
app.use(
  session({
    store: new RedisStore({ client: redisClient }),
    saveUninitialized: false,
    secret: 'local dev secret',
    resave: false,
  })
);


app.use(bodyParser.urlencoded({ extended: false }));
app.use(passport.initialize());
app.use(passport.session());


// render the basic login page
app.get('/login', function(req, res) {
  res.sendFile('login.html', {
    root: __dirname
  });
});

// authenticate the user
app.post('/login',
  passport.authenticate('local', {
    successRedirect: '/',
    failureRedirect: '/login'
  })
);


// create the proxy which adds the special headers
const proxy = httpProxy.createProxyServer({
  target: 'http://localhost:8080/'
});

proxy.on('proxyReq', function(proxyReq, req, res, options) {
  proxyReq.setHeader('X-Waterwheel-User', req.user.username)
})

// use the proxy for all other paths
app.use('/',
  passport.authenticate('session'),
  function(req, res) {
    proxy.web(req, res);
  }
);

// start a server on port 3000
const port = 3000;
app.listen(port, () => {
  console.log(`Waterwheel authn proxy listening at http://localhost:${port}`)
})
