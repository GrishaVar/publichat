main = function() {
  document.getElementById('send_button').onclick = function() {send_message()};
  document.getElementById('join_stop_button').onclick = function() {toggle_loop(); mainloop("");};
  var ws_ip_port = 'ws://' + location.host + "/ws";
  const socket = new WebSocket(ws_ip_port);
  socket.onopen = function() {console.log("socket opened");};
  socket.onerror = function(e) {shutdown(e)};
  socket.onclose = function(e) {shutdown(e)};
  socket.onmessage = function(e) {ws_receive(e)};
  function ws_send(bytes) {
    if (socket.readyState != WebSocket.OPEN) {
      console.log('Tried sending to closed WS');
      loop = false;
      return;
    }
    var outgoing = new Uint8Array(bytes);
    socket.send(outgoing);
  };

  var struct = new JSPack();
  var max_chat_id = Number.MIN_SAFE_INTEGER;
  var min_chat_id = Number.MAX_SAFE_INTEGER;
  var message_byte_size = 172;
  var message_concent_lenght = 128;

  function get_title(){return document.getElementById('title').value;}
  function get_password(){return document.getElementById('password').value;}
  function get_message(){return document.getElementById('message_entry').value;}
  function clear_messages(){document.getElementById('message_list').replaceChildren();}

  var reader = new FileReader();

  // *******************************CHAR_COUNTER*******************************
  var content_div = document.getElementById("message_entry");
  var counter_div = document.getElementById("content_counter");
  content_div.addEventListener("keyup",keystroke_input);
  function keystroke_input(event) {
    // send with enter (enter == 13)
    if(event.keyCode === 13) {send_message();}
    // update colour and value of message length counter
    var textLength = content_div.value.length;
    counter_div.textContent = textLength + "/" + (message_concent_lenght-1);
    if(textLength >= message_concent_lenght-1){
      content_div.style.borderColor = "#ff2851";
      counter_div.style.color = "#ff2851";
    } else{
      content_div.style.borderColor = "#6a197d";
      counter_div.style.color = "#757575";
    }
  };
  // *********************************SHUTDOWN*********************************
  function shutdown(e) {
    loop=false;
    console.log('ws error!'+e.code+e.reason);
    document.getElementById('join_stop_button').style.backgroundColor = "#ef0000";
    document.getElementById('send_button').style.backgroundColor = "#ef0000";
  }

  // *********************************RECEVING*********************************
  reader.onload = function() {
    var result = reader.result;
    var bytes_u8_array = new Uint8Array(result);
    var bytes = Array.from(bytes_u8_array);
    read_message_bytes(bytes);
  };
  function ws_receive(message_event) {
    var blob = message_event.data;
    reader.readAsArrayBuffer(blob);
  };

  function read_message_bytes(bytes) {
    if (bytes == null || bytes == []) {console.log('recevied empty');return;}
    var last_message = null;
    // Checks current scroll height (this needs to be checked BEFORE the message is added)
    var scroll_on_new_msg = (window.innerHeight + window.scrollY) >= document.body.offsetHeight;
    while(bytes.length > 0) {
      var single_message = bytes.splice(0, message_byte_size);
      last_message = bytes_to_message(single_message);
    }
    // scroll to bottom if user is already at bottom
    if (scroll_on_new_msg) {last_message.scrollIntoView();}
  };
  function bytes_to_message(bytes) {
    //message: Message ID, Time, USER ID, Message cypher, Signature
    var unpack = struct.Unpack('I8A32A128A', bytes, 0);
    var message_id = unpack[0];  // load this into a global variable
    var time_bytes = unpack[1];
    var user_id = aesjs.utils.hex.fromBytes(unpack[2]);
    var encrypted_bytes = unpack[3];
    //var Signature = unpack[4];   // veryify this at some point
    max_chat_id = Math.max(max_chat_id, message_id);
    min_chat_id = Math.min(min_chat_id, message_id);
    // username
    var username_string = user_id.slice(0,20); // check if user is the empty hash sha3("")
    if (user_id == "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"){
      username_string = "507550anonymous"; // 507550 is hex for green
    }
    // date string
    var time = struct.Unpack('2L', time_bytes, 0);
    var epoch = 0n;
    for (var i=0; i < time.length; i++) {
      epoch += BigInt(time[i]) << BigInt(((time.length-(i+1)) * 32));
    }
    var date = new Date(Number(epoch));
    var date_string = date.toLocaleString('en-GB', { hour12:false } );
    // message text
    var title = get_title();
    var chat_key = sha3_256.array(title);
    var aes_cnt = new aesjs.ModeOfOperation.ctr(chat_key, new aesjs.Counter(1));
    var padded_decrypted_bytes = aes_cnt.decrypt(encrypted_bytes);
    var decrypted_bytes = padded_decrypted_bytes.slice(0, -padded_decrypted_bytes.slice(-1));
    var message_string = aesjs.utils.utf8.fromBytes(decrypted_bytes);
    return build_message(username_string, date_string, message_string);
  };
  function build_message(username_string, date_string, message_string) {
    let message_list_div = document.getElementById('message_list');
    var msg_div = document.createElement('div');
    var usr_div = document.createElement('div');
    var time_div = document.createElement('div');
    var content_div = document.createElement('div');

    msg_div.className = 'message';
    usr_div.className = 'username';
    time_div.className = 'time';
    content_div.className = 'content';

    usr_div.style.color = "#" + username_string.slice(0,6);
    usr_div.innerHTML = username_string.slice(6);
    time_div.innerHTML = date_string;
    content_div.innerHTML = message_string;

    msg_div.appendChild(usr_div);
    msg_div.appendChild(time_div);
    msg_div.appendChild(content_div);
    message_list_div.appendChild(msg_div);
    return msg_div;
  };
  // *********************************MAINLOOP*********************************
  function mainloop(old_title) {
    var title = get_title();
    if (title == "" || loop == false) {
      setTimeout(function() {mainloop(title);}, 1000);
      return;
    }
    // check if chat title has changed
    if (title == old_title && max_chat_id >= 0) {
      query_messages(title);
    } else {
      // update chat list to new title
      clear_messages();
      max_chat_id = Number.MIN_SAFE_INTEGER;
      min_chat_id = Number.MAX_SAFE_INTEGER;
      fetch_messages(title);
    }
    setTimeout(function() {mainloop(title);}, 500);
  };
  // *********************************BUTTONS*********************************
  function draw_on() {
    var stop_square = document.createElement("div");
    stop_square.id = "stop_square";
    document.getElementById('join_stop_button').replaceChildren(stop_square);
  };
  function draw_off() {
    var join_triangle_top = document.createElement("div");
    var join_triangle_bot = document.createElement("div");
    join_triangle_bot.id = "join_triangle_bot";
    join_triangle_top.id = "join_triangle_top";
    document.getElementById('join_stop_button').replaceChildren(join_triangle_top, join_triangle_bot);
  };
  var loop = false;
  function toggle_loop() {
    if (loop) {
      loop = false;
      draw_off();
    } else {
      loop = true;
      draw_on();
    }
  };
  // *********************************QUERY/FETCH*********************************
  function fetch_messages(title) {
    var chat_key = sha3_256.array(title);
    var chat_id = sha3_256.array(chat_key);
    var outbound_bytes = struct.Pack('3s32A3s', ["fch", chat_id, "end"]);
    ws_send(outbound_bytes);
  };
  function query_messages(title) {
    var chat_key = sha3_256.array(title);
    var chat_id = sha3_256.array(chat_key);
    var query = 0xff_00_00_00 + max_chat_id;
    var outbound_bytes = struct.Pack('3s32AI3s', ["qry", chat_id, query, "end"]);
    ws_send(outbound_bytes);
  };

  // *********************************SENDING*********************************
  function send_message() {
    var outbound_bytes = message_to_bytes();
    if (outbound_bytes == null) {return;}
    document.getElementById('message_entry').value = "";
    ws_send(outbound_bytes);
  };
  function pad_message(message) {
    var message = aesjs.utils.utf8.toBytes(message);
    var pad_lenght = message_concent_lenght - message.length;
    var padding = Array(pad_lenght).fill(pad_lenght);
    // concatinate the arrays
    var padded_message = new Uint8Array(message_concent_lenght);
    padded_message.set(message);
    padded_message.set(padding, message.length);
    return padded_message;
  };
  function message_to_bytes() {
    // message: ["snd", chat_id, user_id, cypher, "end"]
    var title = get_title();        // known by peers
    var password = get_password();  // private
    var message = get_message();    // known by peers
    if (message == "") {return null;}

    var chat_key = sha3_256.array(title);  // known by peers
    var chat_id = sha3_256.array(chat_key);  // chat key is public
    var user_id = sha3_256.array(password);  // user id is public

    var text_bytes = pad_message(message);
    var aes_cnt = new aesjs.ModeOfOperation.ctr(chat_key, new aesjs.Counter(1));
    var encrypted_bytes = aes_cnt.encrypt(text_bytes);

    var pad_start = "snd";
    var pad_end = "end";
    var struct = new JSPack();
    var outbound_bytes = struct.Pack('3s32A32A128A3s', [pad_start, chat_id, user_id, encrypted_bytes, pad_end]);
    return outbound_bytes;
  };
};
