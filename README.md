Felix's QR Code Generator
---
use the -h flag for help


_Short Introduction_
```text
Since the invention of the first QR code, it has become ubiquitous in various services.
The widespread use of QR code is an undeniable reality. Therefore, I undertook this project
to delve into the intricacies behind the black-and-white pixelated matrix we know as the QR code.
```


_How this works_<br/>
---
A brief explanation on how the code works. Look at the source code for a deeper understanding.


The ```text``` is passed as a parameter into the ```Generator``` class.
```rust
pub fn run(self) {...}
```
The function above runs the ```Generator```.

---
<h3>Encoding</h3>

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

---
<h3>Error Correction</h3>

The QR code uses Reed-Solomon error correction.<br/>
Firstly, the data block information is obtained from `BlockDivision` in this format:<br/>
&nbsp; `(b, n)`<br/>
Where:
- `b` is the blocks in this format `(total codewords, data codewords, error capacity)`.<br/>
  To obtain the error codewords in this block: `total codewords - data codewords`
- `n` denotes the number of times each block is repeated.<br/>
  ex: (5, 3) denotes that `block 1` repeats 5 times. It is then followed by `block 2` which repeats 3 times.


Then, the data from `BitStream` is divided into the respective data blocks. If the amount of data codewords does not reach the maximum for the specific version and error correction level of the QR code, `0xEC` and `0x11` are alternately padded at the end.

Afterwards, each data block is passed into the error correction engine. I will not go into the mathematics behind the error correction here.

The final data blocks and error code blocks are then interleaved in this manner:<br/>
$D_{1_{1}}D_{2_{1}}D_{3_{1}}D_{1_{2}}D_{2_{2}}D_{3_{2}}...D_{3_{10}}D_{2_{11}}D_{3_{11}}E_{1_{1}}E_{2_{1}}E_{3_{1}}...$<br/>
$where\text{ }N_{i_{j}}\text{ }i\text{ is the data/error block }i\text{ and j is the }jth\text{ element in data/error block }i$<br/>
This sample also showcases how data blocks of different sizes are handled—namely $D_{2}\text{ and }D_{3}\text{ are both larger than }D_{1}$

---
<h3>Generating the QR code</h3>

This is done in the `QRCode` class.<br/>
Firstly, the matrix is created and the following are added: the finder pattern, the timing pattern, and the alignment pattern.
<br/><br/>
<p align="center">
  <img src="https://github.com/user-attachments/assets/0b58ef94-54d7-4553-ac31-20de1b371f20" width="200" alt="image showing patterns."><br/>
  Link to <a href="https://www.researchgate.net/figure/QR-codes-structure-1-Finder-Pattern-It-is-detecting-position-of-QR-code-structure-is_fig2_295584462">credits</a>
</p>
<br/><br/>
The function `is_occupied` in the `QRCode` class returns true if cell `(x, y)` is on any of the patterns above, including the area for version information and format information.

#
The next step is to copy the data on the QR code matrix. This is done in a single while loop. The pattern is given as follows:
<br/><br/>
<p align="center">
  <img src="https://github.com/user-attachments/assets/8ce62f87-0f2b-48fe-8dd6-335349e2fc48" width="300" alt="image showing data matrix pattern."><br/>
</p>
   
  Look at the code in `qr.rs` to see how the pattern above is programmed.
</p>

#

Then, for each of the mask below, the respective version information and format information is added.
Afterwards, the QR code is masked using 8 mask patterns given below:

<div align="center">
  
  | Mask Pattern | Formula                        |
  |--------------|--------------------------------|
  | 0            | (row + column) % 2 == 0        |
  | 1            | row % 2 == 0                   |
  | 2            | column % 3 == 0                |
  | 3            | (row + column) % 3 == 0        |
  | 4            | (floor(row / 2) + floor(column / 3)) % 2 == 0 |
  | 5            | (row * column) % 2 + (row * column) % 3 == 0 |
  | 6            | ((row * column) % 2 + (row * column) % 3) % 2 == 0 |
  | 7            | ((row + column) % 2 + (row * column) % 3) % 2 == 0 |

</div>
<p align="center">
  <img src="https://github.com/user-attachments/assets/03d51373-2cbf-4b7f-9b3d-1392825efbc2" width="300" alt="image showing masking and format info."><br/>
</p>

A 'bad' QR code may confuse scanners. Therefore, a penalty score is given to each of the 8 masked QR code.<br/>
Given by these rules:
| Rule | Description                                                                 | Penalty |
|------|-----------------------------------------------------------------------------|---------|
| 1    | Too many adjacent modules in the same colour (more than 5 in a row or column) | 5 - i where i is the number of adjacent modules |
| 2    | Blocks of the same colour in 2×2 areas                                        | 3 * i where i is the number of said 2x2 blocks |
| 3    | Patterns that match the finder pattern (like 1:1:3:1:1 ratios)              | 40 * i where i is the number of said patterns |
| 4    | Uneven distribution of dark and light modules (should be close to 50%)      | Let % of light modules be i. The penalty is $10\times2\times floor(\frac{50-i}{5})$ |

Finder patterns, quiet zone, and alignment patterns are not taken into account during the penalty check, given by the `is_reserved` function.<br/>
The mask with the least penalty is chosen as the final QR code.<br/>
The format information is subsequently added in this format:
- 2 bits: Error Correction Level (L=01, M=00, Q=11, H=10)
- 3 bits: Mask Pattern (0–7)
- 10 bits: BCH error correction (which is given as a constant table in `data.rs`)
  → All 15 bits XORed with 101010000010010
  
Look at `qr.rs` for the information on how the format information is laid out.

#
Version Information is only used for QR codes with versions larger or equal to 7.
- 6 bits: Version number
- 12 bits: BCH error correction<br/>
  → All 18 bits encode the version and are placed in fixed areas of the matrix

<p align="center">
  <img src="https://github.com/user-attachments/assets/4b07a557-122f-4d91-9b57-443666f4c743" width="300" alt="image showing masking and format info."><br/>
</p>

Similarly, look at `qr.rs` to know how the version information is laid out and `ecc.rs` for the math behind the BCH error correction codes.

---
Final Thoughts

```
I think the QR code is a great example of just how subtle powerful innovation can be, through everyday use.
Behind its familiar design is a sophisticated interplay of computing and mathematics, particularly in its
use of error correction to ensure its reliability. Its complexity is often overlooked, perhaps because it
works so seamlessly. Yet, taking a moment to understand its inner workings reveals not just a technical feat,
but a quiet testament to human ingenuity. And I think that's truly wonderful.
```
