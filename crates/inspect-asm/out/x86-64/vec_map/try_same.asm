inspect_asm::vec_map::try_same:
	mov rax, rdi
	mov rcx, qword ptr [rsi]
	mov rdx, qword ptr [rsi + 8]
	mov rdi, qword ptr [rsi + 16]
	mov rsi, qword ptr [rsi + 24]
	test rdx, rdx
	jle .LBB0_1
	lea r8, [rcx + 4*rdx]
	mov r9, rcx
.LBB0_0:
	add r9, 4
	cmp r9, r8
	jb .LBB0_0
.LBB0_1:
	movabs r8, 4611686018427387903
	and rdi, r8
	mov qword ptr [rax], rcx
	mov qword ptr [rax + 8], rdx
	mov qword ptr [rax + 16], rdi
	mov qword ptr [rax + 24], rsi
	ret
