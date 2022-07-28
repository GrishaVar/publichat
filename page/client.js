main = function() {
  const landing_page_str = [`
  PubliChat is a semi-private chatting application.
  Chats are encrypted with their title as a key.
  Every chat is accessible to anyone, provided they know the chat's title.
  The title is never sent to the server, so the server can't decrypt the chats.
  This way, the server does not need to be trusted.
  Enter a chat title on the top to fetch messages start reading and
  enter a username and message on the bottom to send something.
  Some example usages of publi.chat is the following.`,
  `Chat securely and privately by picking a secure title
  (like a strong password)
  Make a private note for yourself by picking a secure secret title
  Discuss topics in 'public' chats with insecure titles
  (eg. 'Baking', 'Fishing' or 'Chess')
  Discuss webpages with no comments section (set the title to page's url)`,
];
  const message_byte_size = 512;
  const message_content_length = 396;
  const cypher_length = message_content_length + 4 + 8 + 32;
  const fch_pad = [102,  99, 104];  // "fch"
  const qry_pad = [113, 114, 121];  // "qry"
  const snd_pad = [115, 110, 100];  // "snd"
  const end_pad = [101, 110, 100];  // "end"
  const rcv_pad = [109, 115, 103];  // "msg"
  var max_message_id = Number.MIN_SAFE_INTEGER;
  var min_message_id = Number.MAX_SAFE_INTEGER;
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
  
  const utf8decoder = new TextDecoder();
  const utf8encoder = new TextEncoder();
  var reader = new FileReader();
  var socket = null;
  var loop = false;
  var recv_packets = 0;
  var caught_timeout = false;

  var cur_status;
  const status_data = {
    undefined: ["", [0], ""],
    0: ["--status_wait", [0, 1, 4], "Connecting to server"],
    1: ["--status_ok", [2, 3, 4], "Everything's working fine"],
    2: ["--status_wait", [1, 4], "Fetching paused. Click me to re-enable"],
    3: ["--status_wait", [1, 4], "Not receiving updates from server, check internet connection"],
    4: ["--status_error", [0], "Connection severed. Click me to reconnect"],
  }

  open_socket();

  // *******************************HELPERS************************************
  function get_title(){return document.getElementById("title").value;}
  function get_password(){return document.getElementById("password").value;}
  function get_message(){return message_entry.value;}
  
  function unpack_number(bytes) {
    var res = 0;
    for (var i = 0; i < bytes.length; i++) {
      res *= 256;  // same as res << 8 but also works for number > 32 bit
      res += bytes[i];
    }
    return res;
  };
  function pack_number(num, size) {
    var res = [];
    var num_copy = num;
    for (var i = 0; i < size; i++) {
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
  function set_status(status) {
    let [, succ, ] = status_data[cur_status];
    if (!status in succ) { return; }  // skip illegal transitions

    cur_status = status;
    let [colour, , title] = status_data[status];
    socket_button.style.background = style.getPropertyValue(colour);
    socket_button.title = title;
  };
  function expect_response(identifer) {
    var received_packets = recv_packets;
    // within 2 seconds, recv_packets should have been incremented
    setTimeout(()=>{
      if (
        received_packets == recv_packets  // nothing new has come
        && !caught_timeout  // not already in timeout state
        && loop  // not in paused state
        && socket.readyState == WebSocket.OPEN  // not in shutdown state
      ) {
        set_status(3);
        caught_timeout = true;
        console.log("Response timed out: " + identifer);
      }
    }, 2000);
  };

  // *******************************OPEN_SOCKET********************************
  function open_socket() {
    set_status(0);
    socket = new WebSocket("ws://" + location.host + "/ws");
    socket.onopen = function() {
      console.log("socket opened"); 
      setTimeout(function() {loop = true;}, 1000);
      set_status(1);
    };
    socket.onerror = function(e) {shutdown(e)};
    socket.onclose = function(e) {shutdown(e)};
    socket.onmessage = function(e) {ws_receive(e)};
    reset_chat();

    if (get_title() === "") {
      landing_page();
    }
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
    set_status(4);  // red button top left
    send_button.style.backgroundColor = style.getPropertyValue("--status_err");
    if (typeof e != "string") {console.log("ws error! "+e.code+e.reason);}
    else {console.log(e);}
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
      loop = !loop;
      set_status({true: 1, false: 2}[loop]);
    }
  };

  // *********************************RECEVING*********************************
  function ws_receive(message_event) {
    set_status(1);
    recv_packets += 1;
    caught_timeout = false;

    var blob = message_event.data;
    reader.readAsArrayBuffer(blob);
  };
  reader.onload = function() {
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
    if (chat_id_byte != chat_id_hash[0]) {return;}
    if (message_count*message_byte_size != bytes.length) {return}
    
    if (message_count === 0) {return;}

    if (build_upwards) {
      max_message_id = Math.max(max_message_id, message_id + message_count-1);
      min_message_id = message_id;
    } else {
      max_message_id = message_id + message_count - 1;
      min_message_id = Math.min(min_message_id, message_id);
    }
    
    read_message_bytes(bytes, build_upwards);
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
      return key.verify(hash, signature);
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
  };
  function unpad_message(padded_message, chat_key) {
    var padding_marker = chat_key[0];
    for (var i = padded_message.length - 1; i >= 0; i--) {
      if (padded_message[i] == padding_marker) { break; }
    }
    if (i <= 0) {
      // message 0 length or no pad charachter => error
      console.log("Warning: Message with invalid padding.")
      return [];
    }
    return padded_message.slice(0, i);
  }
  function bytes_to_message(bytes) {
    // Break message server side
    var server_time = unpack_number(bytes.splice(0, 8)); // 8 bytes
    var cypher_block = bytes.splice(0, cypher_length); // 440 needs splicing
    var signature = bytes.splice(0, 64);
    var bytes_hash = sha3_256.array(cypher_block);

    // decrypt message
    var cnt = new aesjs.Counter(1);
    var aes_cnt = new aesjs.ModeOfOperation.ctr(get_chat_key(), cnt);
    var decrypted_bytes = Array.from(aes_cnt.decrypt(cypher_block));

    // Break message client side
    var chat_key_4bytes = decrypted_bytes.splice(0, 4); // 4 bytes
    var client_time = unpack_number(decrypted_bytes.splice(0, 8)); // 8 bytes
    var public_key = decrypted_bytes.splice(0, 32); // 32 bytes
    var padded_bytes = decrypted_bytes.splice(0, message_content_length);// 396
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
    // message string remove padding
    var message_bytes = unpad_message(padded_bytes, chat_key_4bytes);
    var message_str = utf8decoder.decode(new Uint8Array(message_bytes));
    
    var [msg_div, sig_div] = build_msg(username_str, date_str, message_str);
    setTimeout(
      ()=>verify_message(
        sig_div, public_key, bytes_hash, signature, 
        server_time, client_time, chat_key_4bytes
      ), 0
    );  // this is to make the singature checking async to the building of msg
    return msg_div; 
  };
  function build_msg(username_str, date_str, message_str) {
    var msg_div = document.createElement("div");
    var usr_div = document.createElement("div");
    var time_div = document.createElement("div");
    var content_div = document.createElement("div");

    msg_div.className = "message";
    usr_div.className = "username";
    time_div.className = "time";
    content_div.className = "content";

    var bg_colour = "#" + username_str.slice(0,6);
    usr_div.style.background = bg_colour;
    usr_div.style.color = white_or_black(bg_colour);  // selects best contrast
    usr_div.textContent = username_str.slice(6);
    time_div.textContent = date_str;
    content_div.textContent = message_str;
    msg_div.appendChild(usr_div);
    msg_div.appendChild(time_div);
    msg_div.appendChild(content_div);
    return [msg_div, time_div];
  };
  function verify_message(
    time_div, public_key, bytes_hash, signature, 
    server_time, client_time, chat_key_4bytes
  ) {
    // Verifies the time, sign., and chat id
    // also adds checkmark to each message
    let chat_verified, time_verified, sig_verified;
    let reason = `Message verified!`
    var verified = (
      (chat_verified = verify_chat_key(chat_key_4bytes))
      && (time_verified = verify_time(server_time, client_time))
      && (sig_verified = verify_signature(public_key, bytes_hash, signature))
    );
    if (!verified) {
      console.log(
        "Message from: ", aesjs.utils.hex.fromBytes(public_key).slice(0, 20),
        "\nCould not be verified because:",
        "\nChat check: ", chat_verified,
        "\nTime check: ", time_verified,
        "\nSignature check: ", sig_verified,
      );
      if (!chat_verified) {
        reason = "Message sent to wrong chat.\n" +
          "Possible attack, take caution!\n" +
          "An impersonator may have copied this valid message from a different chat.\n" +
          "May have happened if you switched chats too quickly."
      } else if (!time_verified) {
        reason = "Message sent at strange time.\n" +
          "Possible attack, take caution!\n" +
          "An impersonator may have resent an old message from this chat.\n" +
          "May have happened due to poor connection or strange time settings."
      } else if (!sig_verified) {
        reason = "Message signed incorrectly.\n" +
          "Possible attack, take caution!\n" +
          "An impersonator may be failing to impersonate."
      }
    }
    time_div.appendChild(make_verify_mark(verified, reason));
  };
  function make_verify_mark(is_verified, reason) {
    var main_div = document.createElement("div");
    var circle = document.createElement("div");
    var stem = document.createElement("div");
    var kick = document.createElement("div");
    main_div.className = "checkmark";
    circle.className = "checkmark_circle";
    stem.className = "checkmark_stem";
    kick.className = "checkmark_kick";

    var checkmark_colour = "--status_ok";
    if (!is_verified) {
      checkmark_colour = "--status_err";
    }
    circle.style.background = style.getPropertyValue(checkmark_colour);
    main_div.appendChild(circle);
    main_div.appendChild(stem);
    main_div.appendChild(kick);

    main_div.title = reason;
    return main_div;
  };

  // *********************************MAINLOOP*********************************
  function mainloop(old_title) {
    var title = get_title();
    if (title == "") {
      landing_page();
      setTimeout(function() {mainloop(title);}, 1000);
      return;
    }
    if (loop == false) {
      setTimeout(function() {mainloop(title);}, 1000);
      return;
    }
    // check if chat title has changed (and we have received essages)
    if (title == old_title && max_message_id >= min_message_id) {
      query_messages(false);  // false means new messages
    } else {
      // update chat list to new title
      reset_chat();
      fetch_messages();
    }
    setTimeout(function() {mainloop(title);}, 500);
  };
  
  function landing_page() {
    reset_chat();
    for (let msg_str of landing_page_str) {
      var [msg_div, time_div] = build_msg("991133Admin", "2022-03-01 13:37", msg_str);
      time_div.appendChild(make_verify_mark(true));
      message_list_div.appendChild(msg_div);
    }
  }
  // *********************************QUERY/FETCH******************************
  function fetch_messages() {
    var chat_id = get_chat_id();
    chat_id_hash = chat_id;
    ws_send([].concat(fch_pad, chat_id, end_pad));
    expect_response("fetch");
  };
  function query_messages(up) {
    var chat_id = get_chat_id();
    if (up) { // query messages upward (old messages)
      var query = [0x7f].concat(pack_number(min_message_id, 3));
    } else { // query messages downward (new messages)
      var query = [0xff].concat(pack_number(max_message_id, 3));
    }
    ws_send([].concat(qry_pad, chat_id, query, end_pad));
    expect_response("query");
  };
  function top_scroll_query(e) {
    if (message_list_div.scrollTop == 0  && max_message_id > min_message_id) {
      query_messages(get_title(), true);
    }
  };

  // *********************************SENDING**********************************
  function send_message() {
    var chat_id = get_chat_id();
    var cypher = create_cypher_block();
    if (cypher == null) {return;}
    // counter_div.textContent = "0/" + message_content_length;
  
    var signature = sign(cypher);
    outbound_bytes = [].concat(snd_pad, chat_id, cypher, signature, end_pad);

    ws_send(outbound_bytes);
    message_entry.value = "";
  };
  function pad_message(message, chat_key) {
    var message = utf8encoder.encode(message);  // make message into array

    var padding_marker = chat_key[0];
    var padding_length  = message_content_length - message.length;
    var padding = new Uint8Array(padding_length);
    
    var possible_bytes = new Uint8Array(256 - 1);  // each byte 0-254 (no 255)
    for (var i = 0; i < possible_bytes.length; i++) {possible_bytes[i] = i;}  // [0, 1, 2, ... 254]
    possible_bytes[padding_marker] = 255; // exclude padding_marker

    padding[0] = padding_marker;  // First byte should be the padding marker
    for (var i = 1; i < padding_length; i++) {
      var rand_byte = Math.floor(Math.random() * 256)  // evenly distributed on ints [0, 255]  (inclusive)
      padding[i] = possible_bytes[rand_byte];  // on [0, 255] \ {marker}
    }
    
    // GRISHA'S BETTER SOLUTION
    // for (var i = 0; i < padding_length; i++) {
    //   var rand_byte = Math.floor(Math.random() * 255)  // evenly distributed on ints [0, 254]
    //   padding[i] = (rand_byte + (255-padding_marker)) % 256;  // on [0, 255] \ {marker}
    // }

    // concatinate the arrays
    var padded_message = new Uint8Array(message_content_length);
    padded_message.set(message);
    padded_message.set(padding, message.length);
    return padded_message;
  };
  function sign(cypher) {
    //var EdDSA = require('elliptic').eddsa;
    var ec = new elliptic.eddsa('ed25519');
    var secret = get_password();
    var hashed_secret = sha3_256.array(utf8encoder.encode(secret))
    var key_pair = ec.keyFromSecret(hashed_secret);
    var cypher_hash = sha3_256.array(cypher);
    var signature = key_pair.sign(cypher_hash).toBytes();
    return signature;
  };
  function get_public_key() {
    var ec = new elliptic.eddsa('ed25519');
    var secret = get_password();
    var hashed_secret = sha3_256.array(utf8encoder.encode(secret))
    var key_pair = ec.keyFromSecret(hashed_secret);
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
  function create_cypher_block() {
    var message = get_message();    // known by peers
    if (message == "") {return null;}
    var chat_key = get_chat_key();  // known by peers
    var message_data = [].concat(
      chat_key.slice(0,4),        // 4 bytes
      get_time_array(),           // 8 bytes
      get_public_key(),           // 32 bytes
      ...pad_message(message, chat_key)     // 396 bytes
    );
    var cnt = new aesjs.Counter(1);
    var aes_cnt = new aesjs.ModeOfOperation.ctr(chat_key, cnt);
    return Array.from(aes_cnt.encrypt(message_data));
  };

  // *******************************CHAR_COUNTER*******************************
  //var counter_div = document.getElementById("content_counter");
  function keystroke_input(event) {
    // send with enter (enter == 13)
    if(event.keyCode === 13) {send_message();}
    // update colour and value of message length counter
    var textLength = message_entry.value.length;
    //counter_div.textContent = textLength + "/" + (message_content_length-1);
    if(textLength >= message_content_length - 10){
      //sending_div.style.borderColor = style.getPropertyValue("--status_err");
      send_button.style.background = style.getPropertyValue("--status_err");
      //counter_div.style.color = "#ff2851";
    } else {
      //sending_div.style.borderColor = style.getPropertyValue("--borders1");
      send_button.style.background = style.getPropertyValue("--borders1");
      //counter_div.style.color = "#757575";
    }
  };

  mainloop("");
};
