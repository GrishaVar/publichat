main = function() {
  

  var max_message_id = Number.MIN_SAFE_INTEGER;
  var min_message_id = Number.MAX_SAFE_INTEGER;
  var message_byte_size = 168;
  var message_content_lenght = 128;
  var fch_padding = [102,  99, 104];  // "fch"
  var qry_padding = [113, 114, 121];  // "qry"
  var snd_padding = [115, 110, 100];  // "snd"
  var end_padding = [101, 110, 100];  // "end"
  var rcv_padding = [109, 115, 103];  // "msg"
  var chat_id_hash = [];  // hash of current chat id
  var style = getComputedStyle(document.body);
  var send_button = document.getElementById("send_button");
  var socket_button = document.getElementById("socket_button");
  var sending_div = document.getElementById("sending_div");
  var message_entry = document.getElementById("message_entry");
  let message_list_div = document.getElementById("message_list");
  send_button.onclick = function() {send_message()};
  socket_button.onclick = function() {toggle_loop();};

  var reader = new FileReader();
  var socket = null;
  open_socket();

  function get_title(){return document.getElementById("title").value;}
  function get_password(){return document.getElementById("password").value;}
  function get_message(){return message_entry.value;}
  function clear_messages(){message_list_div.replaceChildren();}


  function white_or_black(colour) {
    var r = parseInt(colour.slice(1,3), 16);
    var g = parseInt(colour.slice(3,5), 16);
    var b = parseInt(colour.slice(5,7), 16);
    return ((r*0.299 + g*0.587 + b*0.114) > 150) ? "#000000" : "#ffffff";
  }
  // *******************************SET_STATUS*******************************
  function set_status(value) {
    if (value == 0) { // good = 0; wait = 1; error = 2;
      socket_button.style.background = style.getPropertyValue("--status_ok");
    } else if (value == 1) {
      socket_button.style.background = style.getPropertyValue("--status_wait");
    } else {
      socket_button.style.background = style.getPropertyValue("--status_err");
    }
  }

  // *******************************OPEN_SOCKET*******************************
  function open_socket() {
    set_status(1);
    socket = new WebSocket("ws://" + location.host + "/ws");
    socket.onopen = function() {console.log("socket opened"); set_status(0)};
    socket.onerror = function(e) {shutdown(e)};
    socket.onclose = function(e) {shutdown(e)};
    socket.onmessage = function(e) {ws_receive(e)};
  };
  function ws_send(bytes) {
    if (socket.readyState != WebSocket.OPEN) {
      shutdown("Tried sending to dead socket");
      return;
    }
    var outgoing = new Uint8Array(bytes);
    socket.send(outgoing);
  };
    
  // *******************************UNPACK*******************************
  function unpack_number(bytes) {
    res = 0;
    for (var i = 0; i<bytes.length; i++) {
      res *= 256;  // this is the same as res << 8 but also works for number > 32 bit
      res += bytes[i];
    }
    return res;
  };
  function pack_number(num, size) {
    res = []
    for (var i = 0; i<size; i++) {
      res.unshift(num & 0xff);
      num = num >>> 8;
    }
    if (num > 0) {console.log("warning num did not fit in array size", num, size)}
    return res;
  };

  // *******************************CHAR_COUNTER*******************************
  //var counter_div = document.getElementById("content_counter");
  message_entry.addEventListener("keyup",keystroke_input);
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

  // *******************************TOP_SCROLL_QUERY*******************************   
  message_list_div.addEventListener("scroll", top_scroll_query);
  function top_scroll_query(e) {
    if (message_list_div.scrollTop == 0) {
      query_messages(get_title(), true);
    }
  };

  // *********************************SHUTDOWN*********************************
  function shutdown(e) {
    loop=false;
    if (typeof e != "string") {console.log("ws error! "+e.code+e.reason);}
    else {console.log(e);}
    
    send_button.style.backgroundColor = style.getPropertyValue("--status_err");
    set_status(2);  // red button top left
  };
  
  // *********************************BUTTONS*********************************
  var loop = true;
  function toggle_loop() {
    if (socket.readyState != WebSocket.OPEN) {
      open_socket();
      loop = false;
      setTimeout(function() {loop = true;}, 1000);
      return;
    }

    set_status(Number(loop));
    loop = !loop;
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
    var msg_id = unpack_number(bytes.splice(0, 3));
    var msg_count_and_direction = bytes.splice(0, 1)[0];
    var msg_count = msg_count_and_direction & 0x7f;
    var read_forward = (msg_count_and_direction & 0x80) > 0;

    if (msg_padding[0] != rcv_padding[0]) {shutdown("smrt: incorrect msg padding");}
    if (msg_padding[1] != rcv_padding[1]) {shutdown("smrt: incorrect msg padding");}
    if (msg_padding[2] != rcv_padding[2]) {shutdown("smrt: incorrect msg padding");}
    if (chat_id_byte != chat_id_hash[0]) {set_status(0); return;}
    //if (msg_id > max_message_id+1) {set_status(0); return;}
    //if (msg_id < min_message_id-msg_count) {set_status(0); return;}
    if (msg_count != bytes.length / message_byte_size) {set_status(3); return;}
    
    read_message_bytes(bytes, msg_id, read_forward, msg_count);
    set_status(0);  // green button top left
  };

  function read_message_bytes(bytes, msg_id, read_forward, count) {
    if (bytes == null || bytes == []) {console.log("recevied empty");return;}
    // Checks current scroll height (this needs to be checked BEFORE the message is added)
    var scroll_pos = (message_list_div.scrollTop + message_list_div.clientHeight);
    var scroll_threshold = (message_list_div.scrollHeight * 0.90);
    var scroll_to_message = scroll_pos > scroll_threshold || message_list_div.scrollTop < 10;
    var last_message = null;

    if (read_forward) {   // insert at bottom; read messages normally
      while (bytes.length > 0) {
        var single_message = bytes.splice(0, message_byte_size);
        last_message = bytes_to_message(single_message, msg_id);
        msg_id += 1;
      }
    } else { // insert at top; read messages backwards
      msg_id += count - 1;  // set to top id; decrement as we go
      while (bytes.length > 0) {
        var single_message = bytes.splice(-message_byte_size);
        last_message = bytes_to_message(single_message, msg_id);
        msg_id -= 1;
      }
    }

    // scroll to bottom if user is already at bottom
    if (scroll_to_message && last_message != null) {last_message.scrollIntoView();}
  };
  function bytes_to_message(bytes, message_id) {
    //message: Message ID, Time, USER ID, Message cypher, Signature
    // var message_id = unpack_number(bytes.splice(0, 4)); // 4 bytes
    var time = unpack_number(bytes.splice(0, 8)); // 8 bytes
    var user_id = aesjs.utils.hex.fromBytes(bytes.splice(0, 32)); // 32 bytes
    var encrypted_bytes = bytes.splice(0, 128);
    //var Signature = bytes.splice(0, 128);   // veryify this at some point*/
    // build direction (are messages new or old?)
    var build_upwards = message_id < min_message_id;
    if (!build_upwards && max_message_id > message_id) {
      console.log("Recived non contiguous messages (min-id-max)", min_message_id, message_id, max_message_id); 
      return null;
    }
    max_message_id = Math.max(max_message_id, message_id);
    min_message_id = Math.min(min_message_id, message_id);
    // username
    var username_string = user_id.slice(0,20); // check if user is the empty hash sha3("")
    if (user_id == "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"){
      username_string = "79985aAnonymous"; // 507550 is hex for green
    }
    // date string
    var date = new Date(Number(time));
    var today = new Date();
    if (date.toDateString() === today.toDateString()) {
      var date_string = ""
    } else if (date.getFullYear() === today.getFullYear()){
      var date_string = date.toLocaleString().slice(0,-15);
    } else {
      var date_string = date.toLocaleString().slice(0,-10);
    }
    date_string += " " + date.toLocaleTimeString().slice(0,-3);
    // message text
    var title = get_title();
    var chat_key = sha3_256.array(title);
    var aes_cnt = new aesjs.ModeOfOperation.ctr(chat_key, new aesjs.Counter(1));
    var padded_decrypted_bytes = aes_cnt.decrypt(encrypted_bytes);
    var decrypted_bytes = padded_decrypted_bytes.slice(0, -padded_decrypted_bytes.slice(-1));
    var message_string = aesjs.utils.utf8.fromBytes(decrypted_bytes);
    return build_message(username_string, date_string, message_string, build_upwards);
  };
  function build_message(username_string, date_string, message_string, build_upwards) {
    var msg_div = document.createElement("div");
    var usr_div = document.createElement("div");
    var time_div = document.createElement("div");
    var content_div = document.createElement("div");

    msg_div.className = "message";
    usr_div.className = "username";
    time_div.className = "time";
    content_div.className = "content";

    var bg_colour = "#" + username_string.slice(0,6);
    usr_div.style.background = bg_colour;
    usr_div.style.color = white_or_black(bg_colour);  // selects best contrast
    usr_div.innerHTML = btoa(username_string).slice(0,8);
    time_div.innerHTML = date_string;
    content_div.innerHTML = message_string;

    msg_div.appendChild(usr_div);
    msg_div.appendChild(time_div);
    msg_div.appendChild(content_div);
    if (build_upwards) {
      message_list_div.prepend(msg_div);
    } else {
      message_list_div.appendChild(msg_div);
    }
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
    if (title == old_title && max_message_id >= 0) {
      query_messages(title, false);  // false means new messages
    } else {
      set_status(1);  // yellow button top left will be made green by receive
      // update chat list to new title
      clear_messages();
      max_message_id = Number.MIN_SAFE_INTEGER;
      min_message_id = Number.MAX_SAFE_INTEGER;
      fetch_messages(title);
    }
    setTimeout(function() {mainloop(title);}, 500);
  };
  
  // *********************************SENDING*********************************
  function send_message() {
    var outbound_bytes = message_to_bytes();
    if (outbound_bytes == null) {return;}
    document.getElementById("message_entry").value = "";
    // counter_div.textContent = "0/" + message_content_lenght;
    ws_send(outbound_bytes);
    
  };
  function pad_message(message) {
    var message = aesjs.utils.utf8.toBytes(message);
    var pad_lenght = message_content_lenght - message.length;
    var padding = Array(pad_lenght).fill(pad_lenght);
    // concatinate the arrays
    var padded_message = new Uint8Array(message_content_lenght);
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
    var encrypted_bytes = Array.from(aes_cnt.encrypt(text_bytes));

    return [].concat(snd_padding, chat_id, user_id, encrypted_bytes, end_padding);
  };
  
  // *********************************QUERY/FETCH*********************************
  function fetch_messages(title) {
    var chat_key = sha3_256.array(title);
    var chat_id = sha3_256.array(chat_key);
    chat_id_hash = chat_id;
    ws_send([].concat(fch_padding, chat_id, end_padding));
  };
  function query_messages(title, up) {
    var chat_key = sha3_256.array(title);
    var chat_id = sha3_256.array(chat_key);
    if (up) {  // query messages upward (old messages)
      var query = [0x7f].concat(pack_number(min_message_id, 3));
    } else {  // query messages downward (new messages)
      var query = [0xff].concat(pack_number(max_message_id, 3));
    }
    ws_send([].concat(qry_padding, chat_id, query, end_padding));
  };

  mainloop("");
};
