inspect_asm::alloc_u32_slice_clone::bumpalo:
	push r14
	push rbx
	push rax
	lea rcx, [4*rdx]
	mov r8, qword ptr [rdi + 16]
	mov rax, qword ptr [r8 + 32]
	cmp rcx, rax
	ja .LBB0_2
	sub rax, rcx
	and rax, -4
	cmp rax, qword ptr [r8]
	jb .LBB0_2
	mov qword ptr [r8 + 32], rax
	test rax, rax
	je .LBB0_2
.LBB0_0:
	test rdx, rdx
	je .LBB0_6
	lea rcx, [rdx - 1]
	movabs r8, 4611686018427387903
	and r8, rcx
	xor edi, edi
	cmp r8, 7
	jb .LBB0_3
	mov r9, rax
	sub r9, rsi
	mov rcx, rsi
	cmp r9, 32
	jb .LBB0_4
	inc r8
	mov rdi, r8
	and rdi, -8
	lea rcx, [rsi + 4*rdi]
	xor r9d, r9d
.LBB0_1:
	movups xmm0, xmmword ptr [rsi + 4*r9]
	movups xmm1, xmmword ptr [rsi + 4*r9 + 16]
	movups xmmword ptr [rax + 4*r9], xmm0
	movups xmmword ptr [rax + 4*r9 + 16], xmm1
	add r9, 8
	cmp rdi, r9
	jne .LBB0_1
	cmp r8, rdi
	jne .LBB0_4
	jmp .LBB0_6
.LBB0_2:
	mov rbx, rsi
	mov esi, 4
	mov r14, rdx
	mov rdx, rcx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, rbx
	mov rdx, r14
	test rax, rax
	jne .LBB0_0
	call qword ptr [rip + bumpalo::oom@GOTPCREL]
.LBB0_3:
	mov rcx, rsi
.LBB0_4:
	lea rsi, [rsi + 4*rdx]
	lea rdi, [rax + 4*rdi]
.LBB0_5:
	mov r8d, dword ptr [rcx]
	add rcx, 4
	mov dword ptr [rdi], r8d
	add rdi, 4
	cmp rcx, rsi
	jne .LBB0_5
.LBB0_6:
	add rsp, 8
	pop rbx
	pop r14
	ret
