.TOP.main | ({
  "cycles":.clk | add,

  "out_00": .out_mem["mem(0)(0)"] | .[-1],
  "out_01": .out_mem["mem(0)(1)"] | .[-1],
  "out_02": .out_mem["mem(0)(2)"] | .[-1],
  "out_10": .out_mem["mem(1)(0)"] | .[-1],
  "out_11": .out_mem["mem(1)(1)"] | .[-1],
  "out_12": .out_mem["mem(1)(2)"] | .[-1],
  "out_20": .out_mem["mem(2)(0)"] | .[-1],
  "out_21": .out_mem["mem(2)(1)"] | .[-1],
  "out_22": .out_mem["mem(2)(2)"] | .[-1],

  "pe_00": .pe_0_0.acc.out | unique,
  "pe_01": .pe_0_1.acc.out | unique,
  "pe_02": .pe_0_2.acc.out | unique,
  "pe_10": .pe_1_0.acc.out | unique,
  "pe_11": .pe_1_1.acc.out | unique,
  "pe_12": .pe_1_2.acc.out | unique,
  "pe_20": .pe_2_0.acc.out | unique,
  "pe_21": .pe_2_1.acc.out | unique,
  "pe_22": .pe_2_2.acc.out | unique
})
