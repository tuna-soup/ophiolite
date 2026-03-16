# **LAS Version 2.0: A Digital Standard for Logs Update January 2014**

BY

Canadian Well Logging Society (www.cwls.org)

LAS Committee: C. Struyk, KC Petrophysics Inc J. Karst, Schlumberger Canada Ltd.

## **1.0 Abstract:**

The LAS 2.0 log data standard was introduced in 1992 and continues to be popular. This paper updates the LAS 2.0 documentation and makes a minor change to the LAS 2.0 specifications to better reflect the technological advances made since its introduction.

The changes and clarifications are as follows:

- Line length is unrestricted in unwrapped mode (change)
- The depth value divided by the step value must be a whole number (clarification)
- Rounding of depth values is not acceptable. (clarification)
- The delimiters in a non-comment line are the first dot in the line, the first space after that dot and the last colon in the line. (clarification)
- Most LAS 2.0 files have a depth based index however a time based index is permitted (clarification).

## **2.0 Introduction:**

This paper updates the LAS 2.0 documentation (Log ASCII Standard version 2.0). The updating was necessary to clarify some items not specifically stated in the earlier documentation and to better reflect the technological advances made since its introduction.

The LAS standard was introduced by the Canadian Well Logging Society in 1989 to standardize the organization of digital log curve information for personal computer users. It did this very successfully and the standard became popular worldwide. Version 1.2 was the first version and followed in September 1992 by version 2.0 to address some inconsistencies. A more versatile version LAS 3.0 was released in 1999 however at present, LAS 2.0 remains the dominant product. LAS 3.0 clarifies several of the poorly defined specifications of LAS 2.0 and provides expanded data storage capabilities, but has seen limited implementation.

## **3.0 LAS 2.0 Overview:**

- An LAS file is a structured ASCII file containing log curve data and header information. The header information is located at the beginning of the file and followed by curve data.
- The standard was designed to simplify the exchange of digital log data between users.
- The LAS format is intended for optically presented log curves although other curves may also be included.
- The ASCII character set is limited to ASCII 13 (carriage return), ASCII 10 (line feed), and ASCII 32 to ASCII 126 inclusive. All other ASCII characters are not allowed and it is suggested that software readers convert them to a space (removing them may cause issues if the character was intended to represent a space such as the tab character). Line termination will consist of ASCII 13 ASCII 10 (CR LF) except for the last line.
- Each LAS 2.0 file contains only one continuous interval in the data section. For example, a repeat section would make up one file and the main pass another.
- LAS files end in ".LAS" so that they can be easily recognized.
- Each LAS file consists of sections. Sections begin with a header line defined as beginning with the ~ tilde character when it occurs as the first non-space character on a line. The character immediately following the tilde character defines the section with the remainder of the line being ignored. The characters "V", "W", "C", "P", "O", and "A" are reserved in the LAS 2.0 standard. The sections defined by the LAS 2.0 standard are limited to one occurrence per file. Customer defined sections are permitted but must be located after the first section (~V) and before the last section (~A).
- The sections defined for the LAS 2.0 standard are as follows:
  - "**~V"** (also known as "~VERSION INFORMATION SECTION") is a required section; has formatting requirements; must be the first section; identifies the version number and whether data is in "wrapped" or "un-wrapped" mode.
  - **"~W"** (also known as "WELL INFORMATION SECTION") is a required section; has formatting requirements; is preferably the second section; contains information on the well name, location, and start and stop values of the data in this file.
  - **"~C"** (also known as ~CURVE INFORMATION SECTION") is a required section; has formatting requirements; contains curve mnemonics and their definitions in the order that they appear in the data section.
  - -**"~P"** (also known as ~PARAMETER INFORMATION SECTION") is an optional section; has formatting requirements; contains information on parameters or constants relevant to the wellbore such as mud resistivity, wireline engineer, truck number etc.
  - -**"~O"** (also known as "~OTHER") is an optional section; has no formatting requirements; contains other information or comments.
  - **"~A"** (also known as ~ASCII LOG DATA") is a required section; has formatting requirements; is the last section in the file and also referred to as the data section. The index of the data columns is either Depth or Time. The index values always appear in the first column and each column of data must be separated by at least one space (ASCII 32). All values in the ASCII log data section must be floating point or integer (long) values. Other formats such as Text or Exponential values are not supported.
- Two flags are used in LAS files: 1) "#" signifies a comment line when used as the first non-space character on a line and 2) "~" signifies the beginning of a section when used as the first nonspace character on a line.
- The sections "VERSION", "WELL", "CURVE" and "PARAMETER" use line delimiters. The delimiters are: 1) first dot in a line 2) first space after the first dot in a line and 3) the last colon in a line.
- Example LAS files can be found at the end of this paper.

## **4.0 Software:**

Software exists for LAS data and can be found on the CWLS website (www.cwls.org).

The Certify program was designed to verify that files meet the LAS standard and will identify any errors encountered. The checks are based on structure, not content. That is, it will not flag an empty well name field, but will recognize that required sections are missing or if a line is not structured correctly. In case of disagreement between this program and the printed LAS standard document, the document will be deemed to be correct. The Windows based LAS CERTIFY program was written by J. Karst of Schlumberger.

An LAS utility was written by C. Struyk. The utility includes the following processes:

- 1. reverse depth direction
- 2. convert LAS 1.2 to 2.0 and LAS 3.0 to LAS 2.0
- 3. resample data
- 4. change depth from metres to feet or feet to metres
- 5. fix start depth and step issues
- 6. unwrap LAS files
- 7. wrap LAS files
- 8. Scan and fix some common errors in LAS files
- 9. Merge LAS files
- 10. Convert text files to LAS files
- 11. Check LAS files for errors

The above programs are not part of the LAS standard. The authors of these programs do not reserve any rights and do not warrant the programs for any specific purpose.

## **5.0 Details:**

This portion of the paper provides a detailed look at all of the components of an LAS 2.0 file. Flags and delimiters are discussed first, followed by a discussion of the 'sections' as defined by the LAS 2.0 format. This portion of the paper is best understood by looking at the examples in the boxed areas and the examples provided at the end of this paper.

## **5.1 Flags**

Certain characters are used to assist software in identifying specific lines within a file. The following flags are defined in the LAS 2.0 format:

> "**~**" (tilde): The ASCII equivalent of this flag is decimal 126. This character is recognized as a flag when it occurs as the first non-space character on a line. This flag is used to mark the beginning of a section within an LAS file. The first letter directly after the tilde identifies the section. The upper case letters "V", "W", "P", "C", "O", and "A" in the space following a tilde

mark are reserved for use by the committee. The remainder of the line will be treated as comments.

"**#**" (pound): The ASCII equivalent of this flag is decimal 35. This character is recognized as a flag when it occurs as the first non-space character on a line. This flag is used to indicate that the line is a comment line. Comment lines can appear anywhere above the ~A section.

### **5.2 Line Delimiters**

Three line delimiters are used in the "**~V**", "**~W**", "**~C**", and "**~P**" sections of LAS files. The line delimiters are as follows:

- a) the first dot in a line,
- b) the first space after the first dot in a line
- c) the last colon in a line

All non-comment lines in these sections must contain all three of the above delimiters.

An example line is as follows:

![](_page_3_Figure_9.jpeg)

Where:

**MNEM** = mnemonic. This mnemonic can be of any length but must not contain any internal spaces, dots, or colons. Spaces are permitted in front of the mnemonic and between the end of the mnemonic and the dot.

**UNITS** = units of the mnemonic (if applicable). The units, if used, must be located directly after the dot. There must be no spaces between the units and the dot. The units can be of any length but must not contain any colons or internal spaces.

**DATA** = value of, or data relating to the mnemonic. This value or input can be of any length and can contain spaces, dots or colons as appropriate. It must be preceded by at least one space to demarcate it from the units and must be to the left of the last colon in the line.

**DESCRIPTION** = description or definition of the mnemonic. It is always located to the right of the last colon. The length of the line is no longer limited.

### **5.3 ~V (Version Information)**

- This section is mandatory and must appear as the first section in the file.
- Only one **"~V"** section can occur in an LAS 2.0 file.
- It identifies the version of the LAS format and whether wrap mode is used.
- This section must contain the following lines:

```
 VERS. 2.0 : CWLS LOG ASCII STANDARD - VERSION 2.0
Refers to which version of LAS used.
```

and

or

**WRAP. YES : Multiple lines per depth step**

**WRAP. NO : One line per depth step**

Refers to whether a wrap around mode was used in the data section. If the wrap mode is "NO", there is no limit to the line length. If wrap mode is used, the depth value will be on its own line and all lines of data will be no longer than 80 characters (including carriage return and line feed).

- There is no longer a line length limited on LAS files. The original LAS format was limited to a line length of 256 characters because of early computer limitations. Modern computer equipment does not have an issue with line lengths and therefore the line length limitation has been withdrawn. The updated LAS 2.0 standard retains the "**WRAP YES"** definition as an option and for backwards compatibility.
- Additional lines in the version section are optional.
- The following is an example of a Version Information Section.

|       | ~Version Information Section |     |                                      |
| ----- | ---------------------------- | --- | ------------------------------------ |
| VERS. | 2.0                          | :   | CWLS log ASCII Standard -VERSION 2.0 |
| WRAP. | NO                           | :   | One line per depth step              |

### **5.4 ~W (Well Information)**

- This section is mandatory.
- Only one **"~W"** section can occur in an LAS 2.0 file.
- It identifies the well, its unique location identifier and indicates the start and stop depths (or times) of the file.
- This section must contain the following lines with the mnemonics as indicated:

### **STRT.M nnn.nn : START DEPTH**

Refers to the first depth (or time) in the file. The "nnn.nn" refers to the depth (or time) value. The value must be identical in value to the first depth (time) in the ~ASCII section although its format may vary (123.45 is equivalent to 123.45000).

The number of decimals used is not restricted. If the index is depth, the units must be M (meters), F (feet) or FT (feet). Units must match on the lines relating to STRT, STOP, STEP and the index (first) channel in the ~C section. If time, the units can be any unit that results in a floating point number representation of time. (dd/mm/yy or hh:mm:ss formats are not supported). The logical depth order (shallow to deep or deep to shallow) is optional. Successive time index values must increase if the index is "Time". The start depth (or time) when divided by the step depth (or time) must be a whole number.

#### **STOP.M nnn.n : STOP DEPTH**

Same comments as for STRT except this value represents the LAST data line in the ~ASCII log data section. The stop depth when divided by the step depth must be a whole number.

#### **STEP.M nnn.nn : STEP**

Same comments as for STRT, except this value represents the actual difference between every successive index value in the ~ASCII log data section. The sign (+ or -) represents the logical difference between each successive index value. (+ for increasing index values). The step must be identical in value between every index value throughout the file. If the step increment is not exactly consistent between every index sample, then the step must have a value of 0.

**NULL. nnnn.nn : NULL VALUE** Refers to null values. Commonly used null values are -9999, -999.25 and -9999.25.

**COMP. aaaaaaaaaaaaaaaaaaaaa : COMPANY** Refers to company name.

**WELL. aaaaaaaaaaaaaaaaaaaaa : WELL** Refers to the well name.

**FLD. aaaaaaaaaaaaaaaaaaaaa : FIELD** Refers to the field name.

**LOC. aaaaaaaaaaaaaaaaaaaaa : LOCATION** Refers to the well location.

**PROV. aaaaaaaaaaaaaaaaaaaaa : PROVINCE** Refers to the province. For areas outside Canada this line may be replaced by:

**CNTY. aaaaaaaaaaaaaaaaaaaaa : COUNTY STAT. aaaaaaaaaaaaaaaaaaaaa : STATE CTRY. aaaaaaaaaaaaaaaaaaaaa : COUNTRY**

**SRVC. aaaaaaaaaaaaaaaaaaaaa : SERVICE COMPANY** Refers to logging company.

**DATE. aaaaaaaaaaaaaaaaaaaaa : DATE** Refers to date logged. The preferred data is of the form yyyy mm dd

**UWI . aaaaaaaaaaaaaaaaaaaaa : UNIQUE WELL ID** Refers to unique well identifier. Within Canada, the most common UWI consists of a 16 character string. Please exclude all dashes, slashes and spaces from such UWIs.

For areas outside of Canada this may be replaced by:

#### **API . aaaaaaaaaaaaaaaaaaaaa : API NUMBER**

- Additional lines in the well information section are optional. There is no limit on the number of additional lines.
  **LIC. nnnnnn : LICENCE NUMBER** Refers to a regulatory licence number. Required by ERCB in Alberta

- The following is an example of a Well Information Section in LAS version 2.0:

|                           |                          | **\*\***\*\*\*\***\*\***\*\***\*\***\*\*\*\***\*\***\_**\*\***\*\*\*\***\*\***\*\***\*\***\*\*\*\***\*\***                    |     |     |     |     |     |     |     |
| ------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------- | --- | --- | --- | --- | --- | --- | --- |
| ~Well Information Section |                          |                                                                                                                               |     |     |     |     |     |     |     |
| #MNEM.UNIT                | VALUE/NAME               | DESCRIPTION                                                                                                                   |     |     |     |     |     |     |     |
| #--------                 | --------------           | ---------------------                                                                                                         |     |     |     |     |     |     |     |
| STRT.M                    | 635.0000                 | :START DEPTH                                                                                                                  |     |     |     |     |     |     |     |
| STOP.M                    | 400.0000                 | :STOP DEPTH                                                                                                                   |     |     |     |     |     |     |     |
| STEP.M                    | -0.125                   | :STEP                                                                                                                         |     |     |     |     |     |     |     |
| NULL.                     | -999.25                  | :NULL VALUE                                                                                                                   |     |     |     |     |     |     |     |
| COMP.                     | ANY OIL COMPANY INC.     | :COMPANY                                                                                                                      |     |     |     |     |     |     |     |
| WELL.                     | ANY ET AL 12-34-12-34    | :WELL                                                                                                                         |     |     |     |     |     |     |     |
| FLD .                     | WILDCAT                  | :FIELD                                                                                                                        |     |     |     |     |     |     |     |
| LOC .                     | 12-34-12-34W5M           | :LOCATION                                                                                                                     |     |     |     |     |     |     |     |
| PROV.                     | ALBERTA                  | :PROVINCE                                                                                                                     |     |     |     |     |     |     |     |
| SRVC.                     | ANY LOGGING COMPANY INC. | :SERVICE COMPANY                                                                                                              |     |     |     |     |     |     |     |
| LIC                       | 12345                    | :ERCB LICENCE NUMBER                                                                                                          |     |     |     |     |     |     |     |
| DATE.                     | 13-DEC-86                | :LOG DATE                                                                                                                     |     |     |     |     |     |     |     |
| UWI .                     | 100123401234W500         | :UNIQUE WELL ID<br>**\*\***\*\*\*\***\*\***\*\***\*\***\*\*\*\***\*\***\_**\*\***\*\*\*\***\*\***\*\***\*\***\*\*\*\***\*\*** |     |     |     |     |     |     |     |

### **5.5 ~C (Curve Information)**

- This section is mandatory.
- Only one **"~C"** section can occur in an LAS 2.0 file.
- It describes the curves and its units in the order they appear in the ~ASCII log data section of the file.
- The mnemonics used are not restricted but must be defined on the line in which they appear.
- API curve codes are optional. (May be required by some regulatory agencies)
- The channels described in this section must be present in the data set.
- The first channel described is the index of all other channels, and is either Depth or Time. The only valid mnemonics for the index channel are DEPT, DEPTH or TIME.

|                        | **\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***\_**\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***<br>~Curve Information Section |     |                |                   |     |     |     |                                       |     |     |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | --- | -------------- | ----------------- | --- | --- | --- | ------------------------------------- | --- | --- |
| #MNEM.UNIT<br>API CODE |                                                                                                                                                          |     |                | Curve Description |     |     |     |                                       |     |     |
|                        | #------------------                                                                                                                                      |     |                |                   |     |     |     | ----------------<br>----------------- |     |     |
| DEPT                   | .M                                                                                                                                                       |     |                |                   |     | :   | 1   | DEPTH                                 |     |     |
| RHOB                   | .K/M3                                                                                                                                                    |     | 45 350 02 00 : |                   |     |     | 2   | BULK DENSITY                          |     |     |
| NPH                    | .VOL/VO                                                                                                                                                  |     | 42 890 00 00 : |                   |     |     | 3   | NEUTRON POROSITY -<br>SANDSTONE       |     |     |
| MSFL                   | .OHMM                                                                                                                                                    |     | 20 270 01 00 : |                   |     |     | 4   | Rxo RESISTIVITY                       |     |     |
|                        | SFLA .OHMM                                                                                                                                               |     | 07 222 01 00 : |                   |     |     | 5   | SHALLOW RESISTIVITY                   |     |     |
| ILM                    | .OHMM                                                                                                                                                    |     | 07 120 44 00 : |                   |     |     | 6   | MEDIUM RESISTIVITY                    |     |     |
| ILD                    | .OHMM                                                                                                                                                    |     | 07 120 46 00 : |                   |     |     | 7   | DEEP RESISTIVITY                      |     |     |
| SP                     | .MV                                                                                                                                                      |     | 07 010 01 00 : |                   |     |     | 8   | SPONTANEOUS POTENTIAL                 |     |     |
| GR                     | .GAPI                                                                                                                                                    |     | 45 310 01 00 : |                   |     |     | 9   | GAMMA RAY                             |     |     |
| CALI .MM               |                                                                                                                                                          |     | 45 280 01 00 : |                   |     |     | 10  | CALIPER                               |     |     |

The following is an example of a Curve Information Section with API codes.

#### **5.6 ~P (Parameter Information)**

- This section is optional. It defines the input values of various parameters relating to this well. These input values can consist of numbers or words.
- Only one **"~P"** section can occur in an LAS 2.0 file.
- The mnemonics used are not restricted but must be defined on the line on which they appear.
- There is no limit on the number of lines that can be used.
- The following is an example of a Parameter Information Section.

|                     |            |                                |              |     | **\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***\_\_\_\_**\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\*** |     |     |     |     |     |
| ------------------- | ---------- | ------------------------------ | ------------ | --- | -------------------------------------------------------------------------------------------------------------------------------- | --- | --- | --- | --- | --- |
|                     |            | ~Parameter Information Section |              |     |                                                                                                                                  |     |     |     |     |     |
| #MNEM.UNIT<br>Value |            |                                |              |     | Description                                                                                                                      |     |     |     |     |     |
|                     |            | #-----------------             | ------------ |     | ----------------------                                                                                                           |     |     |     |     |     |
| MUD                 |            |                                | GEL CHEM     | :   | Mud type                                                                                                                         |     |     |     |     |     |
| BHT                 | .DEGC      |                                | 114.0000     | :   | Bottom Hole Temperature                                                                                                          |     |     |     |     |     |
| BS                  | .MM        |                                | 222.0000     | :   | Bit Size                                                                                                                         |     |     |     |     |     |
| CSGL                | .M         |                                | 345.7        | :   | Casing Depth                                                                                                                     |     |     |     |     |     |
| FD                  | .K/M3      |                                | 999.9999     | :   | Fluid Density                                                                                                                    |     |     |     |     |     |
| MDEN                | .K/M3      |                                | 2650.0000    | :   | Logging Matrix Density                                                                                                           |     |     |     |     |     |
| MATR .              |            |                                | SAND         | :   | Neutron Matrix                                                                                                                   |     |     |     |     |     |
| FNUM .              |            |                                | 1.0000       | :   | Tortuosity Const. Archie's(a)                                                                                                    |     |     |     |     |     |
| FEXP .              |            |                                | 2.000        | :   | Cementation Exp<br>Archie's (m)                                                                                                  |     |     |     |     |     |
| DFD                 | .K/M3      |                                | 1200.0000    | :   | Mud Weight                                                                                                                       |     |     |     |     |     |
| DFV                 | .S         |                                | 50.0000      | :   | Mud Viscosity                                                                                                                    |     |     |     |     |     |
| DFL                 | .C3        |                                | 8.0000       | :   | Mud Fluid Loss                                                                                                                   |     |     |     |     |     |
| DFPH .              |            |                                | 10.00        | :   | Mud pH                                                                                                                           |     |     |     |     |     |
|                     | RMFS .OHMM |                                | 2.8200       | :   | Mud Filtrate Resistivity                                                                                                         |     |     |     |     |     |
| EKB                 | .M         |                                | 566.9700     | :   | Elevation Kelly Bushing                                                                                                          |     |     |     |     |     |
| EGL                 | .M         |                                | 563.6799     | :   | Elevation Ground Level                                                                                                           |     |     |     |     |     |
|                     |            |                                |              |     | **\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***\_\_\_\_**\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\***\*\***\*\*\*\*** |     |     |     |     |     |

### **5.7 ~O (Other Information)**

- This section is optional. It is intended as a remarks or comments section.
- Only one **"~O"** section can occur in an LAS 2.0 file.
- This section has no delimiter requirements.
- The following is an example of an "Other Information Section"

---

#### ------------------------------------------------------------------------------------------------------------ **~Other Information Section**

**The log digits for this well were hand digitized from poor half scale log prints. This was the best information available at the time. Every attempt should be made to track down the original films. .Dec. 12,1990 John Doe, Petrophysics**

## **5.8 ~A (ASCII Log Data)**

- The data section will always be the last section in a file.
- Only one **"~A"** section can occur in an LAS 2.0 file.
- Embedded blank lines anywhere in the section are forbidden
- Each column of data must be separated by at least one space. Consistency of format on every line, while not required, is expected by many LAS readers. Right Justification of each column of data and the same width of all data fields is highly recommended.
- Line length in the data section of unwrapped files are no longer restricted
- In wrap mode, the index channel will be on its own line
- In wrap mode, a line of data will be no longer than 80 characters. This includes a carriage return and line feed.

## **6.0 References**

- C. Struyk, R. Bishop, D. Fortune, E. Foster, D. Gordon, T. d'Haene, D. Joyce, S. Kenny, H. Kowalchuk and M. Stadnyk, 1989; LAS, A Floppy Disk Standard For Log Data, Canadian Well Logging Society, 12th Formation Evaluation Symposium , Paper J ; The Log Analyst, V30,No.5 P 395-396; Geobyte 1989.
  CWLS Floppy Disk Committee; 1992; LAS 2.0, A Floppy Disk Standard For Log Data; [www.cwls.org.](http://www.cwls.org/)

- CWLS LAS Committee; 2009, LAS Version 2.0 Updated: July 2009, A digital Standard for Logs; www.cwls.org

## Example #1 - LAS 2.0 in Unwrapped Mode

|             | ~VERSION INFORMATION                                  |                              |                         |              |                                                 |                                                                |     |     |     |
| ----------- | ----------------------------------------------------- | ---------------------------- | ----------------------- | ------------ | ----------------------------------------------- | -------------------------------------------------------------- | --- | --- | --- |
| VERS.       |                                                       | 2.0 :                        |                         |              | CWLS LOG ASCII STANDARD -VERSION 2.0            |                                                                |     |     |     |
| WRAP.       |                                                       | NO<br>:                      | ONE LINE PER DEPTH STEP |              |                                                 |                                                                |     |     |     |
|             | ~WELL INFORMATION                                     |                              |                         |              |                                                 |                                                                |     |     |     |
| #MNEM.UNIT  |                                                       | DATA                         |                         |              | DESCRIPTION                                     |                                                                |     |     |     |
| #-----      | -----                                                 | ----------                   |                         |              | -----------------                               |                                                                |     |     |     |
| STRT        | .M                                                    | 1670.0000                    |                         | :START DEPTH |                                                 |                                                                |     |     |     |
| STOP        | .M                                                    | 1669.7500                    |                         | :STOP DEPTH  |                                                 |                                                                |     |     |     |
| STEP        | .M                                                    | -0.1250                      |                         | :STEP        |                                                 |                                                                |     |     |     |
| NULL        |                                                       | -999.25                      |                         | :NULL VALUE  |                                                 |                                                                |     |     |     |
| COMP        |                                                       | ANY OIL COMPANY INC.         |                         | :COMPANY     |                                                 |                                                                |     |     |     |
| WELL        |                                                       | ANY ET AL 12-34-12-34        |                         | :WELL        |                                                 |                                                                |     |     |     |
| FLD         | WILDCAT                                               |                              |                         | :FIELD       |                                                 |                                                                |     |     |     |
| LOC         |                                                       | 12-34-12-34W5M               |                         | :LOCATION    |                                                 |                                                                |     |     |     |
| PROV        | ALBERTA                                               |                              |                         |              | :PROVINCE                                       |                                                                |     |     |     |
| SRVC        |                                                       | ANY LOGGING COMPANY INC.     |                         |              |                                                 | :SERVICE COMPANY                                               |     |     |     |
| DATE        |                                                       | 13-DEC-86                    |                         |              | :LOG DATE                                       |                                                                |     |     |     |
| UWI         |                                                       | 100123401234W500             |                         |              | :UNIQUE WELL ID                                 |                                                                |     |     |     |
| LIC         | 23412                                                 |                              |                         |              |                                                 | :ERCB LICENCE NUMB                                             |     |     |     |
|             | ~CURVE INFORMATION                                    |                              |                         |              |                                                 |                                                                |     |     |     |
| #MNEM.UNIT  |                                                       | API CODES                    |                         |              |                                                 | CURVE DESCRIPTION                                              |     |     |     |
|             | #------------------                                   | ------------                 |                         |              |                                                 | -------------------                                            |     |     |     |
| DEPT        | .M                                                    |                              |                         | :<br>1       | DEPTH                                           |                                                                |     |     |     |
| DT          | .US/M                                                 | 60 520 32 00                 |                         | :<br>2       |                                                 | SONIC TRANSIT TIME                                             |     |     |     |
| RHOB        | .K/M3                                                 | 45 350 01 00                 |                         | :<br>3       | BULK DENSITY                                    |                                                                |     |     |     |
| NPHI        | .V/V                                                  | 42 890 00 00                 |                         | :<br>4       |                                                 | NEUTRON POROSITY                                               |     |     |     |
| SFLU        | .OHMM                                                 | 07 220 04 00                 |                         | :<br>5       |                                                 | SHALLOW RESISTIVITY                                            |     |     |     |
| SFLA        | .OHMM                                                 | 07 222 01 00                 |                         | :<br>6       |                                                 | SHALLOW RESISTIVITY                                            |     |     |     |
| ILM         | .OHMM                                                 | 07 120 44 00                 |                         | :<br>7       |                                                 | MEDIUM RESISTIVITY                                             |     |     |     |
| ILD         | .OHMM                                                 | 07 120 46 00                 |                         | :<br>8       |                                                 | DEEP RESISTIVITY                                               |     |     |     |
|             |                                                       |                              |                         |              |                                                 |                                                                |     |     |     |
| #MNEM.UNIT  | ~PARAMETER INFORMATION                                | VALUE                        |                         | DESCRIPTION  |                                                 |                                                                |     |     |     |
|             |                                                       |                              |                         |              |                                                 |                                                                |     |     |     |
| MUD         | #--------------                                       | ----------------<br>GEL CHEM | :                       | MUD TYPE     |                                                 | ------------------------                                       |     |     |     |
|             |                                                       |                              |                         |              |                                                 |                                                                |     |     |     |
| BHT         | .DEGC                                                 | 35.5000                      | :                       |              | BOTTOM HOLE TEMPERATURE                         |                                                                |     |     |     |
| CSGL        | .M                                                    | 124.6                        | :                       |              | BASE OF CASING                                  |                                                                |     |     |     |
| MATR        |                                                       | SAND                         | :                       |              | NEUTRON MATRIX                                  |                                                                |     |     |     |
| MDEN        |                                                       | 2710.0000                    | :                       |              | LOGGING MATRIX DENSITY                          |                                                                |     |     |     |
| RMF         | .OHMM                                                 | 0.2160                       | :                       |              | MUD FILTRATE RESISTIVITY<br>DRILL FLUID DENSITY |                                                                |     |     |     |
| DFD         | .K/M3                                                 | 1525.0000                    | :                       |              |                                                 |                                                                |     |     |     |
| ~OTHER      |                                                       |                              |                         |              |                                                 |                                                                |     |     |     |
|             | data between 625 metres and 615 metres to be invalid. |                              |                         |              |                                                 | Note: The logging tools became stuck at 625 metres causing the |     |     |     |
| #           |                                                       |                              |                         |              |                                                 |                                                                |     |     |     |
| ~A<br>DEPTH | DT                                                    | RHOB                         | NPHI<br>SFLU            |              | SFLA                                            | ILM<br>ILD                                                     |     |     |     |
| 1670.000    | 123.450 2550.000                                      | 0.450                        | 123.450                 |              | 123.450                                         | 110.200<br>05.600                                              |     |     |     |
| 1669.875    | 123.450 2550.000                                      | 0.450                        | 123.450                 |              | 123.450                                         | 110.200<br>05.600                                              |     |     |     |
| 1669.750    | 123.450 2550.000                                      | 0.450                        | 123.450                 |              | 123.450                                         | 110.200 105.600                                                |     |     |     |

## **Example #2 - LAS 2.0 With Minimal Header Information in Unwrapped Mode.**

|          |                       |                                                                |                                                           |                                                        |                                                                                                              |                                                                                 |                                                                                                                     | SANDSTONE                                                                                                                                                                                                                                                                                                                                             |
| -------- | --------------------- | -------------------------------------------------------------- | --------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|          |                       |                                                                |                                                           |                                                        |                                                                                                              |                                                                                 |                                                                                                                     |                                                                                                                                                                                                                                                                                                                                                       |
|          |                       |                                                                |                                                           |                                                        |                                                                                                              |                                                                                 |                                                                                                                     |                                                                                                                                                                                                                                                                                                                                                       |
|          |                       |                                                                |                                                           |                                                        |                                                                                                              |                                                                                 |                                                                                                                     |                                                                                                                                                                                                                                                                                                                                                       |
|          |                       |                                                                |                                                           |                                                        |                                                                                                              |                                                                                 |                                                                                                                     |                                                                                                                                                                                                                                                                                                                                                       |
|          |                       |                                                                |                                                           |                                                        |                                                                                                              |                                                                                 |                                                                                                                     |                                                                                                                                                                                                                                                                                                                                                       |
|          |                       |                                                                |                                                           |                                                        |                                                                                                              |                                                                                 |                                                                                                                     |                                                                                                                                                                                                                                                                                                                                                       |
|          |                       |                                                                |                                                           |                                                        |                                                                                                              |                                                                                 |                                                                                                                     |                                                                                                                                                                                                                                                                                                                                                       |
| 634.8750 |                       |                                                                |                                                           |                                                        |                                                                                                              |                                                                                 |                                                                                                                     |                                                                                                                                                                                                                                                                                                                                                       |
|          | .M<br>.MV<br>635.0000 | 23412<br>.K/M3<br>.VOL/VOL<br>.OHMM<br>.OHMM<br>.OHMM<br>.OHMM | 2.0<br>NO<br>WILDCAT<br>ALBERTA<br>13-DEC-86<br>2256.0000 | :<br>:<br>12-34-12-34W5M<br>100123401234W500<br>0.4033 | ANY OIL COMPANY INC.<br>ANY ET AL 12-34-12-34<br>:<br>:<br>:<br>:<br>:<br>:<br>:<br>:<br>2256.0000<br>0.4033 | 635.0000<br>634.8750<br>-0.1250<br>-999.25<br>ANY LOGGING COMPANY INC.<br>DEPTH | CWLS LAS-VERSION 2.0<br>:STEP<br>:COMPANY<br>:WELL<br>:FIELD<br>BULK DENSITY<br>Rxo RESISTIVITY<br>DEEP RESISTIVITY | One line per depth step<br>:START DEPTH<br>:STOP DEPTH<br>:NULL VALUE<br>:LOCATION<br>:PROVINCE<br>:SERVICE COMPANY<br>:LOG DATE<br>:UNIQUE WELL ID<br>:ERCB LICENCE NUMB<br>NEUTRON POROSITY -<br>SHALLOW RESISTIVITY<br>MEDIUM RESISTIVITY<br>SPONTANEOUS POTENTIAL<br>22.0781 22.0781 20.3438 3.6660 123.4<br>22.0781 22.0781 20.3438 3.6660 123.4 |

## **Example #3 – LAS 2.0 Wrapped Version**

|            | ~VERSION INFORMATION |                          |            |     |      |                                         |     |
| ---------- | -------------------- | ------------------------ | ---------- | --- | ---- | --------------------------------------- | --- |
| VERS.      |                      | 2.0                      | :          |     |      | CWLS log ASCII Standard<br>-VERSION 2.0 |     |
| WRAP.      |                      | YES                      | :          |     |      | Multiple lines per depth step           |     |
|            | ~WELL INFORMATION    |                          |            |     |      |                                         |     |
| #MNEM.UNIT |                      |                          | DATA       |     |      | DESCRIPTION                             |     |
| #-----     | -----                |                          | ---------- |     |      | -----------------------                 |     |
| STRT       | .M                   |                          | 910.0000   |     |      | :START DEPTH                            |     |
| STOP       | .M                   |                          | 909.5000   |     |      | :STOP DEPTH                             |     |
| STEP       | .M                   |                          | -0.1250    |     |      | :STEP                                   |     |
| NULL       |                      |                          | -999.25    |     |      | :NULL VALUE                             |     |
| COMP       |                      | ANY OIL COMPANY INC.     |            |     |      | :COMPANY                                |     |
| WELL       |                      | ANY ET AL 12-34-12-34    |            |     |      | :WELL                                   |     |
| FLD        |                      | WILDCAT                  |            |     |      | :FIELD                                  |     |
| LOC        |                      | 12-34-12-34W5M           |            |     |      | :LOCATION                               |     |
| PROV       |                      | ALBERTA                  |            |     |      | :PROVINCE                               |     |
| SRVC       |                      | ANY LOGGING COMPANY INC. |            |     |      | :SERVICE COMPANY                        |     |
| SON        |                      | 142085                   |            |     |      | :SERVICE ORDER NUMBER                   |     |
| DATE       |                      | 13-DEC-86                |            |     |      | :LOG DATE                               |     |
| UWI        |                      | 100123401234W500         |            |     |      | :UNIQUE WELL ID                         |     |
| LIC        |                      | 23412                    |            |     |      | :LICENCE NUMB.                          |     |
|            | ~CURVE INFORMATION   |                          |            |     |      |                                         |     |
| #MNEM.UNIT |                      |                          |            |     |      | Curve Description                       |     |
| #--------- |                      |                          |            |     |      | ----------------------------            |     |
| DEPT       | .M                   |                          |            | :   |      | Depth                                   |     |
| DT         | .US/M                |                          |            | :   |      | 1 Sonic Travel Time                     |     |
| RHOB       | .K/M                 |                          |            | :   |      | 2 Density-Bulk Density                  |     |
| NPHI       | .V/V                 |                          |            | :   |      | 3 Porosity -Neutron                     |     |
| RX0        | .OHMM                |                          |            | :   |      | 4 Resistivity -Rxo                      |     |
| RESS       | .OHMM                |                          |            | :   |      | 5 Resistivity -Shallow                  |     |
| RESM       | .OHMM                |                          |            | :   |      | 6 Resistivity -Medium                   |     |
| RESD       | .OHMM                |                          |            | :   |      | 7 Resistivity -Deep                     |     |
| SP         | .MV                  |                          |            | :   |      | 8 Spon. Potential                       |     |
| GR         | .GAPI                |                          |            | :   |      | 9 Gamma Ray                             |     |
| CALI       | .MM                  |                          |            |     |      | : 10 Caliper                            |     |
| DRHO       | .K/M3                |                          |            |     |      | : 11 Delta-Rho                          |     |
| EATT       | .DBM                 |                          |            |     |      | : 12 EPT Attenuation                    |     |
| TPL        | .NS/M                |                          |            |     |      | : 13 TP -EPT                            |     |
| PEF        |                      |                          |            |     |      | : 14 PhotoElectric Factor               |     |
| FFI        | .V/V                 |                          |            |     |      | : 15 Porosity -NML FFI                  |     |
| DCAL       | .MM                  |                          |            |     |      | : 16 Caliper-Differential               |     |
| RHGF       | .K/M3                |                          |            |     |      | : 17 Density-Formation                  |     |
| RHGA       | .K/M3                |                          |            |     |      | : 18 Density-Apparent                   |     |
| SPBL       | .MV                  |                          |            |     |      | : 19 Baselined SP                       |     |
| GRC        | .GAPI                |                          |            |     |      | : 20 Gamma Ray BHC                      |     |
| PHIA       | .V/V                 |                          |            |     |      | : 21 Porosity -Apparent                 |     |
| PHID       | .V/V                 |                          |            |     | : 22 | Porosity -Density                       |     |
| PHIE       | .V/V                 |                          |            |     |      | : 23 Porosity -Effective                |     |
| PHIN       | .V/V                 |                          |            |     |      | : 24 Porosity -Neut BHC                 |     |
|            |                      |                          |            |     |      |                                         |     |

PHIC .V/V : 25 Porosity -Total HCC R0 .OHMM : 26 Ro RWA .OHMM : 27 Rfa SW . : 28 Sw -Effective MSI . : 29 Sh Idx -Min BVW . : 30 BVW FGAS . : 31 Flag -Gas Index PIDX . : 32 Prod Idx FBH . : 33 Flag -Bad Hole FHCC . : 34 Flag -HC Correction LSWB . : 35 Flag -Limit SWB ~A Log data section 910.000000 -999.2500 2692.7075 0.3140 19.4086 19.4086 13.1709 12.2681 -1.5010 96.5306 204.7177 30.5822 -999.2500 -999.2500 3.2515 -999.2500 4.7177 3025.0264 3025.0264 -1.5010 93.1378 0.1641 0.0101 0.1641 0.3140 0.1641 11.1397 0.3304 0.9529 0.0000 0.1564 0.0000 11.1397 0.0000 0.0000 0.0000 909.875000 -999.2500 2712.6460 0.2886 23.3987 23.3987 13.6129 12.4744 -1.4720 90.2803 203.1093 18.7566 -999.2500 999.2500 3.7058 -999.2500 3.1093 3004.6050 3004.6050 -1.4720 86.9078 0.1456 -0.0015 0.1456 0.2886 0.1456 14.1428 0.2646 1.0000 0.0000 0.1456 0.0000 14.1428 0.0000 0.0000 0.0000 909.750000 -999.2500 2692.8137 0.2730 22.5909 22.5909 13.6821 12.6146 -1.4804 89.8492 201.9287 3.1551 -999.2500 -999.2502 4.3124 -999.2500 1.9287 2976.4451 2976.4451 -1.4804 86.3465 0.1435 0.0101 0.1435 0.2730 0.1435 14.5674 0.2598 1.0000 0.0000 0.1435 0.0000 14.5674 0.0000 0.0000 0.0000 909.625000 -999.2500 2644.3650 0.2765 18.4831 18.4831 13.4159 12.6900 -1.5010 93.3999 201.5826 -6.5861 -999.2500 -999.2500 4.3822 -999.2500 1.5826 2955.3528 2955.3528 -1.5010 89.7142 0.1590 0.0384 0.1590 0.2765 0.1590 11.8600 0.3210 0.9667 0.0000 0.1538 0.0000 11.8600 0.0000 0.0000 0.0000 909.500000 -999.2500 2586.2822 0.2996 13.9187 13.9187 12.9195 12.7016 -1.4916 98.1214 201.7126 -4.5574 -999.2500 -999.2500 3.5967 -999.2500 1.7126 2953.5940 2953.5940 -1.4916 94.2670 0.1880 0.0723 0.1880 0.2996 0.1880 8.4863 0.4490 0.8174 0.0000 0.1537 0.0000 8.4863 0.0000 0.0000 0.0000

## **Example # 4 LAS 2.0 Time Based Data**

~VERSION INFORMATION VERS. 2.0 : CWLS LOG ASCII STANDARD -VERSION 2.0 WRAP. NO : ONE LINE PER TIME STEP # ~WELL INFORMATION STRT .S 0.0000 :START TIME STOP .S 1.5000 :STOP TIME STEP .S 0.3000 :STEP NULL . -999.25 :NULL VALUE COMP . ANY OIL COMPANY INC. :COMPANY WELL . ANY ET 12-34-12-34 :WELL FLD . WILDCAT :FIELD LOC . 12-34-12-34W5 :LOCATION PROV . ALBERTA :PROVINCE SRVC . ANY LOGGING COMPANY INC. :SERVICE COMPANY DATE . 13-DEC-86 :LOG DATE UWI . 100123401234W500 :UNIQUE WELL ID # ~CURVE INFORMATION ETIM .S : 1 ELAPSED TIME BFR1 .OHMM : 2 SINGLE PROBE 1 RESISTIVITY BSG1 .PSIG : 3 SINGLE PROBE 1 STRAIN GAUGE PRESSURE # ~PARAMETER INFORMATION MRT .DEGC 67.0 : BOTTOM HOLE TEMPERATURE GDEPT .M 3456.5 : GAUGE DEPTH DFD .KG/M3 1000.0 : MUD WEIGHT # ~A 0.0000 0.2125 16564.1445 0.3000 0.2125 16564.1445 0.6000 0.2125 16564.2421 0.9000 0.2125 16564.0434 1.2000 0.2125 16564.0430 1.5000 0.2125 16564.0435
