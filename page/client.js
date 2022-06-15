main = function() {
  var max_message_id = Number.MIN_SAFE_INTEGER;
  var min_message_id = Number.MAX_SAFE_INTEGER;
  var message_byte_size = 512;
  var message_content_lenght = 396;
  var cypher_length = message_content_lenght + 4 + 8 + 32;
  var fch_pad = [102,  99, 104];  // "fch"
  var qry_pad = [113, 114, 121];  // "qry"
  var snd_pad = [115, 110, 100];  // "snd"
  var end_pad = [101, 110, 100];  // "end"
  var rcv_pad = [109, 115, 103];  // "msg"
  var chat_id_hash = [];  // hash of current chat id
  var style = getComputedStyle(document.body);
  var send_button = document.getElementById("send_button");
  var socket_button = document.getElementById("socket_button");
  var sending_div = document.getElementById("sending_div");
  var message_entry = document.getElementById("message_entry");
  let message_list_div = document.getElementById("message_list");
  send_button.onclick = function() {send_message()};
  socket_button.onclick = function() {toggle_loop();};
  message_list_div.addEventListener("scroll", top_scroll_query);
  message_entry.addEventListener("keyup", keystroke_input);
  
  var reader = new FileReader();
  var socket = null;
  var loop = false;
  open_socket();

  // *******************************HELPERS************************************
  function get_title(){return document.getElementById("title").value;}
  function get_password(){return document.getElementById("password").value;}
  function get_message(){return message_entry.value;}
  
  function unpack_number(bytes) {
    var res = 0;
    for (var i = 0; i<bytes.length; i++) {
      res *= 256;  // same as res << 8 but also works for number > 32 bit
      res += bytes[i];
    }
    return res;
  };
  function pack_number(num, size) {
    var res = [];
    var num_copy = num;
    for (var i = 0; i<size; i++) {
      res.unshift(num & 0xff);
      num = Math.floor(num / 256); // same as >> 8 but works for ints > 32 bit
    }
    if (num > 0) {
      console.log("warning: num too big for array", num_copy, size)
    }
    return res;
  };
  function white_or_black(colour) {  // which text colour gives more contrast
    var r = parseInt(colour.slice(1,3), 16);
    var g = parseInt(colour.slice(3,5), 16);
    var b = parseInt(colour.slice(5,7), 16);
    return ((r*0.299 + g*0.587 + b*0.114) > 150) ? "#000000" : "#ffffff";
  }

  // *******************************SET_STATUS*********************************
  function set_status(value) {
    if (value == 0) { // good = 0; wait = 1; error = 2;
      socket_button.style.background = style.getPropertyValue("--status_ok");
    } else if (value == 1) {
      socket_button.style.background = style.getPropertyValue("--status_wait");
    } else {
      socket_button.style.background = style.getPropertyValue("--status_err");
    }
  };

  // *******************************OPEN_SOCKET********************************
  function open_socket() {
    set_status(1);
    socket = new WebSocket("ws://" + location.host + "/ws");
    socket.onopen = function() {
      console.log("socket opened"); 
      setTimeout(function() {loop = true;}, 1000);
      set_status(0);
    };
    socket.onerror = function(e) {shutdown(e)};
    socket.onclose = function(e) {shutdown(e)};
    socket.onmessage = function(e) {ws_receive(e)};
    reset_chat();
  };
  function ws_send(bytes) {
    if (socket.readyState != WebSocket.OPEN) {
      shutdown("Tried sending to dead socket");
      return;
    }
    var outgoing = new Uint8Array(bytes);
    socket.send(outgoing);
  };
  
  // *********************************SHUTDOWN/RESET***************************
  function shutdown(e) {
    loop = false;
    if (typeof e != "string") {console.log("ws error! "+e.code+e.reason);}
    else {console.log(e);}
    send_button.style.backgroundColor = style.getPropertyValue("--status_err");
    set_status(2);  // red button top left
  };
  function reset_chat(){
    message_list_div.replaceChildren();
    max_message_id = Number.MIN_SAFE_INTEGER;
    min_message_id = Number.MAX_SAFE_INTEGER;
  };
  // *********************************BUTTONS**********************************
  function toggle_loop() {
    if (socket.readyState != WebSocket.OPEN) {
      loop = false;
      open_socket();
    } else {
      set_status(Number(loop));
      loop = !loop;
    }
  };
  // *********************************RECEVING*********************************
  function ws_receive(message_event) {
    var blob = message_event.data;
    reader.readAsArrayBuffer(blob);
  };
  reader.onload = function() {
    set_status(1);  // yellow button top left
    var result = reader.result;
    var bytes_u8_array = new Uint8Array(result);
    var bytes = Array.from(bytes_u8_array);
    // read packet header
    var msg_padding = bytes.splice(0, 3);
    var chat_id_byte = bytes.splice(0, 1);
    var message_id = unpack_number(bytes.splice(0, 3));
    var message_count_and_direction = bytes.splice(0, 1)[0];
    var message_count = message_count_and_direction & 0x7f;
    var build_upwards = (message_count_and_direction & 0x80) == 0;

    if (msg_padding[0] != rcv_pad[0]) {shutdown("incorrect smrt pad 1");}
    if (msg_padding[1] != rcv_pad[1]) {shutdown("incorrect smrt pad 2");}
    if (msg_padding[2] != rcv_pad[2]) {shutdown("incorrect smrt pad 3");}
    if (chat_id_byte != chat_id_hash[0]) {set_status(0); return;}
    if (message_count*message_byte_size != bytes.length) {set_status(0);return}
    
    if (message_count === 0) {set_status(0); return;}

    if (build_upwards) {
      max_message_id = Math.max(max_message_id, message_id + message_count-1);
      min_message_id = message_id;
    } else {
      max_message_id = message_id + message_count - 1;
      min_message_id = Math.min(min_message_id, message_id);
    }
    
    read_message_bytes(bytes, build_upwards);
    set_status(0);  // green button top left
  };

  function read_message_bytes(bytes, build_upwards) {
    if (bytes == null || bytes == []) {console.log("recevied empty");return;}
    // Checks current scroll height BEFORE the message is added
    var scroll_pos = message_list_div.scrollTop+message_list_div.clientHeight;
    var scroll_down = scroll_pos > (message_list_div.scrollHeight * 0.90);
    var scroll_up = message_list_div.scrollTop < 10;
    var scroll_target = null;

    if (build_upwards) { // insert at top; read messages backwards
      scroll_target = message_list_div.children[0];
      while (bytes.length > 0) {
        var single_message = bytes.splice(-message_byte_size);
        new_message_div = bytes_to_message(single_message);
        message_list_div.prepend(new_message_div);
      }
    } else { // insert at bottom; read messages normally
      while (bytes.length > 0) {
        var single_message = bytes.splice(0, message_byte_size);
        new_message_div = bytes_to_message(single_message);
        message_list_div.appendChild(new_message_div);
        scroll_target = new_message_div;
      }
    }
    // scroll to bottom if user is already at bottom
    if ((scroll_down || scroll_up) && scroll_target != null) {
      scroll_target.scrollIntoView();
    }
  };
  function verify_signature(pub_key_bytes, hash, signature) {
    var ec = new elliptic.eddsa('ed25519');
    var key = ec.keyFromPublic(pub_key_bytes, 'bytes');
    try {
      key.verify(hash, signature);
      return true;
    } catch(e) {
      return false;
    }
  };
  function verify_time(server_time, client_time) {
    // server & client time stamp can have a max of 10 seconds difference
    var res = Math.abs(server_time-client_time) < 1000*10;
    return res;
  };
  function verify_chat_key(chat_key_4bytes) {
    var expected = get_chat_key().splice(0,4);
    var res = true;
    for (let i = 0; i < chat_key_4bytes.length; i++) {
      res = res && (expected[i] == chat_key_4bytes[i]);
    }
    return res;
  }
  function make_verify_mark(is_verified) {
    var main_div = document.createElement("div");
    var circle = document.createElement("div");
    var stem = document.createElement("div");
    var kick = document.createElement("div");
    main_div.className = "checkmark";
    circle.className = "checkmark_circle";
    stem.className = "checkmark_stem";
    kick.className = "checkmark_kick";
    var checkmark_colour = "--status_err"
    if (is_verified) {
      checkmark_colour = "--status_ok"
    }
    circle.style.background = style.getPropertyValue(checkmark_colour);
    main_div.appendChild(circle);
    main_div.appendChild(stem);
    main_div.appendChild(kick);
    return main_div;
  };
  function bytes_to_message(bytes) {
    var server_time = unpack_number(bytes.splice(0, 8)); // 8 bytes
    var bytes_hash = sha3_256.array(bytes.slice(0, cypher_length));
    var chat_key_4bytes = bytes.splice(0, 4); // 4 bytes
    var client_time = unpack_number(bytes.splice(0, 8)); // 8 bytes
    var public_key = bytes.splice(0, 32); // 32 bytes
    var encrypted_bytes = bytes.splice(0, message_content_lenght);
    var signature = bytes.splice(0, 64);
    // username string
    var username_str = aesjs.utils.hex.fromBytes(public_key).slice(0, 20);
    if (username_str == "e0b1fe74117e1b95b608") { // pub key of empty string
      username_str = "79985aAnonymous"; // 507550 is hex for green
    }
    // date string
    var date = new Date(Number(server_time));
    var today = new Date();
    if (date.toDateString() === today.toDateString()) {  // sent today
      var date_str = "";
    } else if (date.getFullYear() === today.getFullYear()) {  // sent this year
      var date_str = date.toLocaleString().slice(0,-15);
    } else {
      var date_str = date.toLocaleString().slice(0,-10);  // date < this year
    }
    date_str += " " + date.toLocaleTimeString().slice(0,-3);
    // message string
    var chat_key = get_chat_key();
    var cnt = new aesjs.Counter(1);
    var aes_cnt = new aesjs.ModeOfOperation.ctr(chat_key, cnt);
    var padded_bytes = aes_cnt.decrypt(encrypted_bytes);
    var decrypted_bytes = padded_bytes.slice(0, -2*padded_bytes.slice(-1));
    var message_str = aesjs.utils.utf8.fromBytes(decrypted_bytes);
    
    var sig_verified = verify_signature(public_key, bytes_hash, signature);
    var time_verified = verify_time(server_time, client_time);
    var chat_verified =  verify_chat_key(chat_key_4bytes);
    var is_verified = false;
    if (sig_verified && time_verified && chat_verified) {
      is_verified = true;
    } else {
      console.log(
        "Message: ", message_str,
        "\nfrom: ", username_str,
        "\nCould not be verified because:",
        "\nSignautre check: ", sig_verified,
        "\nTime check: ", time_verified,
        "\nChat check: ", chat_verified,
      );
    }
    return build_message(username_str, date_str, message_str, is_verified);
  };
  function build_message(username_str, date_str, message_str, is_verified) {
    var msg_div = document.createElement("div");
    var usr_div = document.createElement("div");
    var time_div = document.createElement("div");
    var content_div = document.createElement("div");
    var checkmark_div = make_verify_mark(is_verified);

    msg_div.className = "message";
    usr_div.className = "username";
    time_div.className = "time";
    content_div.className = "content";

    var bg_colour = "#" + username_str.slice(0,6);
    usr_div.style.background = bg_colour;
    usr_div.style.color = white_or_black(bg_colour);  // selects best contrast
    usr_div.innerHTML = username_str.slice(6);
    time_div.innerHTML = date_str;
    time_div.appendChild(checkmark_div);
    content_div.innerHTML = message_str;
    msg_div.appendChild(usr_div);
    msg_div.appendChild(time_div);
    msg_div.appendChild(content_div);
    return msg_div;
  };
  // *********************************MAINLOOP*********************************
  function mainloop(old_title) {
    var title = get_title();
    if (title == "" || loop == false) {
      setTimeout(function() {mainloop(title);}, 1000);
      return;
    }
    // check if chat title has changed (and we have received essages)
    if (title == old_title && max_message_id >= min_message_id) {
      query_messages(false);  // false means new messages
    } else {
      set_status(1);  // yellow button top left will be made green by receive
      // update chat list to new title
      reset_chat();
      fetch_messages();
    }
    setTimeout(function() {mainloop(title);}, 500);
  };
  
  // *********************************QUERY/FETCH******************************
  function fetch_messages(title) {
    var chat_id = get_chat_id();
    chat_id_hash = chat_id;
    ws_send([].concat(fch_pad, chat_id, end_pad));
  };
  function query_messages(up) {
    var chat_id = get_chat_id();
    if (up) { // query messages upward (old messages)
      var query = [0x7f].concat(pack_number(min_message_id, 3));
    } else { // query messages downward (new messages)
      var query = [0xff].concat(pack_number(max_message_id, 3));
    }
    ws_send([].concat(qry_pad, chat_id, query, end_pad));
  };
  function top_scroll_query(e) {
    if (message_list_div.scrollTop == 0  && max_message_id > min_message_id) {
      query_messages(get_title(), true);
    }
  };

  // *********************************SENDING**********************************
  function send_message() {
    var chat_id = get_chat_id();
    var cypher = message_to_cypher();
    if (cypher == null) {return;}
    // counter_div.textContent = "0/" + message_content_lenght;
  
    var signature = sign(cypher);
    outbound_bytes = [].concat(snd_pad, chat_id, cypher, signature, end_pad);

    ws_send(outbound_bytes);
    document.getElementById("message_entry").value = "";
  };
  function pad_message(message) {
    if (message.length % 2 == 1) { // add space for message of odd length
      message += ' ';
    }
    var message = aesjs.utils.utf8.toBytes(message);
    
    var pad_lenght = message_content_lenght - message.length;
    var pad_character = Math.floor(pad_lenght/2);
    var padding = Array(pad_lenght).fill(pad_character);
    // concatinate the arrays
    var padded_message = new Uint8Array(message_content_lenght);
    padded_message.set(message);
    padded_message.set(padding, message.length);
    return padded_message;
  };
  function sign(cypher) {
    //var EdDSA = require('elliptic').eddsa;
    var ec = new elliptic.eddsa('ed25519');
    var secret = get_password();
    var key_pair = ec.keyFromSecret(secret);
    var cypher_hash = sha3_256.array(cypher);
    var signature = key_pair.sign(cypher_hash).toBytes();
    return signature;
  };
  function get_public_key() {
    var ec = new elliptic.eddsa('ed25519');
    var secret = get_password();
    var key_pair = ec.keyFromSecret(secret);
    return key_pair.pubBytes();
  };
  function get_time_array() {
    const time = Date.now();
    return pack_number(time, 8);
  };
  function get_chat_key() {
    var title = get_title();
    return sha3_256.array(title);
  }
  function get_chat_id() {
    return sha3_256.array(get_chat_key());
  };
  function message_to_cypher() {
    var message = get_message();    // known by peers
    if (message == "") {return null;}
    var text_bytes = pad_message(message);
    var chat_key = get_chat_key();
    var cnt = new aesjs.Counter(1);
    var aes_cnt = new aesjs.ModeOfOperation.ctr(chat_key, cnt);
    var encrypted_message = Array.from(aes_cnt.encrypt(text_bytes));
    // other stuff
    var chat_key_4bytes = chat_key.slice(0,4);  // known by peers
    var client_time = get_time_array();
    var public_key = get_public_key();
    return [].concat(
      chat_key_4bytes,
      client_time, 
      public_key, 
      encrypted_message
    );
  };

  // *******************************CHAR_COUNTER*******************************
  //var counter_div = document.getElementById("content_counter");
  function keystroke_input(event) {
    // send with enter (enter == 13)
    if(event.keyCode === 13) {send_message();}
    // update colour and value of message length counter
    var textLength = message_entry.value.length;
    //counter_div.textContent = textLength + "/" + (message_content_lenght-1);
    if(textLength >= message_content_lenght-1){
      sending_div.style.borderColor = style.getPropertyValue("--status_err");
      send_button.style.background = style.getPropertyValue("--status_err");
      //counter_div.style.color = "#ff2851";
    } else {
      sending_div.style.borderColor = style.getPropertyValue("--borders1");
      send_button.style.background = style.getPropertyValue("--borders1");
      //counter_div.style.color = "#757575";
    }
  };

  mainloop("");
};
