Felix's QR Code generator
---
use the -h flag for help


_Short Intro_
```text
Since the invention of the first QR Code, it has become ubiquitous in various services.
The widespread use of QR Code is an undeniable reality. Therefore, I undertook this project
to delve into the intricacies behind the black-and-white pixelated matrix we know as the QR Code.
```


_How this works_<br/>
---
A brief explanation on how the code works. Look at the source code for a deeper understanding.


The ```text``` is passed as a parameter into the ```Generator``` class.
```rust
pub fn run(self) {...}
```
The function above runs the ```Generator```.


Since the QR code has 40 different versions, the input is fed into the ```get_version``` function
to find the minimum version required.<br/>
_Dynamic programming is used to find the version.<br/>_
- ```dp[n][mode]``` where _n is the number of characters + 1_ and _mode which denotes the mode used for the character_,
the minimum size required (in bits) to store the characters in ```input[n..]``` using the mode given by ```mode```.
- ```next[n][mode]```, the next set of character and modes. This is used to construct the path from the first character (index `0`) to the last character (index `n-1`).
The path is denoted by a list of ```(pos, mode)``` where `pos` is where the index would change mode from the previous mode into `mode`.
However, a change is unnecessary, or, in other words, no change of modes may happen.
Therefore, the path constructed is processed into ```encoding``` as a list of ```(len, mode)``` where `len` is the number of characters using `mode`.
- Before returning, two things are done:
  - All characters encoded in bytes are taken and passed into an encoder for `ISO-8859-1` (or in the code, `WINDOWS-1252` since they are equivalent).
    If the test fails, an ECI header will be added to set the encoding as `UTF-8 (0111 00011010)`
  - The final addition of length sizes is added since the process above only adds the mode indicator and the size of the data.
- `get_version` returns `(version, encoding, add_eci_utf8)` where `version` is the version of the QR code and `add_eci_utf8` is the flag to add the `UTF-8 ECI header`.<br/>

`encoding` is then iterated and the data is encoded into the `BitStream`.

To be continued: error handling using Reed-Solomon codes and generating the QR code.
