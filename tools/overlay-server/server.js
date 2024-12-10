var express 	= require('express'),
	http 		= require('http'),
	Stopwatch 	= require('./models/stopwatch');

var app = express();
var server = http.createServer(app);
var io = require('socket.io').listen(server);

var bug = {};
var boxing = {lancScore: 0, yorkScore: 0, currRound: ''};
var score = {};
var football = {lancScore: 0, yorkScore: 0};
var dart = {};
var swimming = {order: ''};
var grid = {};

//Clock Functions
var stopwatch = new Stopwatch();

stopwatch.on('tick:stopwatch', function(time) {
	io.sockets.emit("clock:tick", time);
});



io.on('connection', function(socket) {
	console.log("Client Socket Connected");

	/*
	 * 		Clock functions
	 */
	socket.on("clock:pause", function() {
		stopwatch.pause();
	});

	socket.on("clock:reset", function() {
		stopwatch.reset();
	});

	socket.on("clock:up", function() {
		stopwatch.countUp();
	});

	socket.on("clock:down", function() {
		stopwatch.countDown();
	});

	socket.on("clock:set", function(msg) {
		stopwatch.setValue(msg);
	});

    socket.on("clock:get", function() {
        io.sockets.emit("clock:tick", stopwatch.getTime());
    });

		socket.on("grid", function(payload) {
        grid = payload;
        io.sockets.emit("grid", payload);
        console.log("Updating: grid");
    });

	/*
	 * 		General Functions
	 */
	socket.on("bug", function(msg) {
        bug = msg;
		io.sockets.emit("bug", msg);
	});

    socket.on("bug:get", function(msg) {
		io.sockets.emit("bug", bug);
	});

	/*
	 * 		Lower Thirds
	 */
	socket.on("lowerthird:left", function(msg) {
		io.sockets.emit("lowerthird:left", msg);
	});

	socket.on("lowerthird:right", function(msg) {
		io.sockets.emit("lowerthird:right", msg);
	});

	socket.on("lowerthird:hide", function() {
		io.sockets.emit("lowerthird:hide");
	});

	/*
	 * 		Boxing
	 */
	socket.on("boxing", function(msg) {
        boxing = msg;
		io.sockets.emit("boxing", msg);
	});

    socket.on("boxing:get", function(msg) {
		io.sockets.emit("boxing", boxing);
	});

	/*
	 * 		Roses Score
	 */
	socket.on("score", function(msg) {
        score = msg;
		io.sockets.emit("score", msg);
	});

    socket.on("score:get", function(msg) {
		io.sockets.emit("score", score);
	});

	 /*
	 * 		Football
	 */
	socket.on("football", function(msg) {
        football = msg;
		io.sockets.emit("football", msg);
	});

    socket.on("football:get", function(msg) {
		io.sockets.emit("football", football);
	});


	/*
	 * 		Darts
	 */
	socket.on("dart", function(msg) {
        dart = msg;
		io.sockets.emit("dart", msg);
	});

    socket.on("dart:get", function(msg) {
        io.sockets.emit("dart", dart);
    });

    /*
	 * 		Swimming
	 */
	socket.on("swimming", function(msg) {
        swimming = msg;

        swimming.order = (swimming.order).replace(/[^1-8]+/, '');
        swimming.order = (swimming.order).replace(/(.).*\1/, function (x) {return x.substring(0, x.length - 1)})

        if(!('pos1name' in swimming) && swimming.order != '') {
            swimming.splittime = stopwatch.getTime().replace(/^0/, '');
        }

        for(i = 1; i <= 8; i++){
            swimming['pos' + i + 'name'] = eval('swimming.lane' + (swimming.order).charAt(i-1) + 'name');
            swimming['pos' + i + 'team'] = eval('swimming.lane' + (swimming.order).charAt(i-1) + 'team');
            swimming['pos' + i + 'lane'] = (swimming.order).charAt(i-1);
        }

		io.sockets.emit("swimming", msg);
	});

    socket.on("swimming:get", function(msg) {
        io.sockets.emit("swimming", swimming);
    });
});

//Serve the puplic dir
app.use(express.static(__dirname + "/public"));

server.listen(3000);
console.log("Now listening on port 3000. Go to localhost:3000/admin to control")
console.log("run 'play 1-1 [html] http://127.0.0.1:3000' in CasparCG to start the graphics")
