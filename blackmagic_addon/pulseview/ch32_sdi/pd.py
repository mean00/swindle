import sigrokdecode as srd

class Decoder(srd.Decoder):
    api_version = 3
    id = 'ch32_sdi'
    name = 'CH32 SDI'
    longname = 'WCH CH32V SDI (1-wire debug)'
    desc = '1-wire debug protocol for WCH CH32V MCUs'
    license = 'gplv2+'
    inputs = ['logic']
    outputs = ['ch32_sdi']
    
    channels = (
        {'id': 'sdi', 'name': 'SDI', 'desc': 'Data line'},
    )
    
    options = (
        {'id': 't_ns', 'desc': 'Base clock period T (ns)', 'default': 125},
    )

    annotations = (
        ('bit', 'Bit'),
        ('stop', 'Stop Bit'),
        ('error', 'Error'),
        ('header', 'Header'),
        ('data', 'Data'),
    )
    annotation_rows = (
        ('bits', 'Bits', (0, 1, 2)),
        ('fields', 'Fields', (3, 4)),
    )
    
    def __init__(self):
        self.reset()
        
    def reset(self):
        self.samplerate = None
        self.falling_edge = None
        self.rising_edge = None
        self.bits = []
        
    def start(self):
        self.out_ann = self.register(srd.OUTPUT_ANN)
        
    def metadata(self, key, value):
        if key == srd.SRD_CONF_SAMPLERATE:
            self.samplerate = value
            
    def decode(self):
        while True:
            # Wait for falling edge
            try:
                pins = self.wait({0: 'f'})
            except Exception:
                # End of trace
                self.parse_frame()
                return

            falling = self.samplenum
            
            # If we were high for > 16T before this falling edge, it's a new frame
            if self.rising_edge is not None:
                high_samples = falling - self.rising_edge
                high_time = high_samples / self.samplerate
                T = self.options['t_ns'] * 1e-9
                if high_time >= 16 * T:
                    self.put(self.rising_edge, self.rising_edge + int(18*T*self.samplerate), self.out_ann, [1, ['STOP', 'S']])
                    self.parse_frame()
            
            self.falling_edge = falling
            
            # Wait for rising edge
            try:
                pins = self.wait({0: 'r'})
            except Exception:
                return

            self.rising_edge = self.samplenum
            
            low_samples = self.rising_edge - self.falling_edge
            low_time = low_samples / self.samplerate
            
            T = self.options['t_ns'] * 1e-9
            
            bit_val = None
            if 0.5 * T <= low_time <= 5 * T:
                self.put(self.falling_edge, self.rising_edge, self.out_ann, [0, ['1']])
                bit_val = 1
            elif 5 * T <= low_time <= 70 * T:
                self.put(self.falling_edge, self.rising_edge, self.out_ann, [0, ['0']])
                bit_val = 0
            else:
                self.put(self.falling_edge, self.rising_edge, self.out_ann, [2, ['ERR', 'E']])
                
            if bit_val is not None:
                self.bits.append((bit_val, self.falling_edge, self.rising_edge))
                
    def parse_frame(self):
        if not self.bits:
            return
            
        if len(self.bits) >= 9:
            header_bits = self.bits[:9]
            h_val = 0
            for b, _, _ in header_bits:
                h_val = (h_val << 1) | b
                
            start_bit = (h_val >> 8) & 1
            tgt = (h_val >> 1) & 0x7F
            mode = h_val & 1
            mode_str = "READ" if mode == 0 else "WRITE"
            
            self.put(header_bits[0][1], header_bits[-1][2], self.out_ann, [3, [f'HDR: Addr=0x{tgt:02X} {mode_str}', f'A={tgt:02X} {mode_str[0]}']])
            
            if len(self.bits) == 9 + 32:
                data_bits = self.bits[9:]
                d_val = 0
                for b, _, _ in data_bits:
                    d_val = (d_val << 1) | b
                self.put(data_bits[0][1], data_bits[-1][2], self.out_ann, [4, [f'DATA: 0x{d_val:08X}', f'{d_val:08X}']])
            elif len(self.bits) > 9:
                # Incomplete or over-long data
                data_bits = self.bits[9:]
                self.put(data_bits[0][1], data_bits[-1][2], self.out_ann, [4, [f'DATA: {len(data_bits)} bits', f'{len(data_bits)}b']])
            
        self.bits = []
