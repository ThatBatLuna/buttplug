"use strict";

var write_to_textarea = function (str) {
  document.getElementById('log').value = document.getElementById('log').value + str + "\n";
};

var ButtplugClient = {
  socket: undefined,
  connect: function () {
    //get a reference to the element
    var hostInput = document.getElementById('host');
    this.socket = new WebSocket(hostInput.value);
    this.socket.onerror = function (err) {
      write_to_textarea(err);
    };
    this.socket.onmessage = function(msg) {
      write_to_textarea(msg.data);
    };
  },
  getServerInfo: function () {
    if (this.socket === undefined) {
      console.log("Must connect before getting server info!");
    }
    var info_msg = { "Client" : { "RequestServerInfo" : {}}};
    this.socket.send(JSON.stringify(info_msg));
  },
  openDevice: function () {
    if (this.socket === undefined) {
      console.log("Must connect before getting server info!");
    }
    var info_msg = { "Device" : [1, { "ClaimDevice" : {}}]};
    this.socket.send(JSON.stringify(info_msg));
  },
  setVibrateSpeed: function () {
    if (this.socket === undefined) {
      console.log("Must connect before getting server info!");
    }
    var speedValue = document.getElementById('speed').value;
    var info_msg = { "Device" : [1, { "SingleVibrateSpeed" : { "speed": speedValue}}]};
    this.socket.send(JSON.stringify(info_msg));
  }
};

var init_client = function() {
  //get a reference to the element
  var connectBtn = document.getElementById('connect');

  //add event listener
  connectBtn.addEventListener('click', function(event) {
    ButtplugClient.connect();
  });

  var serverInfoBtn = document.getElementById('serverinfo');
  //add event listener
  serverInfoBtn.addEventListener('click', function(event) {
    ButtplugClient.getServerInfo();
  });

  var claimDeviceBtn = document.getElementById('claimdevice');
  //add event listener
  claimDeviceBtn.addEventListener('click', function(event) {
    ButtplugClient.openDevice();
  });

  var speedRange = document.getElementById('speed');
  speedRange.addEventListener('input', function(event) {
    ButtplugClient.setVibrateSpeed();
  });
};

