inspect_asm::alloc_u32_slice_clone::try_bumpalo:
	push r14
	push rbx
	push rax
	lea rcx, [4*rdx]
	mov r8, qword ptr [rdi + 16]
	mov rax, qword ptr [r8 + 32]
	cmp rcx, rax
	ja .LBB_6
	sub rax, rcx
	and rax, -4
	cmp rax, qword ptr [r8]
	jb .LBB_6
	mov qword ptr [r8 + 32], rax
.LBB_3:
	test rdx, rdx
	je .LBB_14
	lea rcx, [rdx - 1]
	movabs r8, 4611686018427387903
	and r8, rcx
	xor edi, edi
	cmp r8, 7
	jb .LBB_5
	mov r9, rax
	sub r9, rsi
	mov rcx, rsi
	cmp r9, 32
	jb .LBB_12
	inc r8
	mov rdi, r8
	and rdi, -8
	lea rcx, [rsi + 4*rdi]
	xor r9d, r9d
.LBB_10:
	movups xmm0, xmmword ptr [rsi + 4*r9]
	movups xmm1, xmmword ptr [rsi + 4*r9 + 16]
	movups xmmword ptr [rax + 4*r9], xmm0
	movups xmmword ptr [rax + 4*r9 + 16], xmm1
	add r9, 8
	cmp rdi, r9
	jne .LBB_10
	cmp r8, rdi
	jne .LBB_12
.LBB_14:
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_5:
	mov rcx, rsi
.LBB_12:
	lea rsi, [rsi + 4*rdx]
	lea rdi, [rax + 4*rdi]
.LBB_13:
	mov r8d, dword ptr [rcx]
	add rcx, 4
	mov dword ptr [rdi], r8d
	add rdi, 4
	cmp rcx, rsi
	jne .LBB_13
	jmp .LBB_14
.LBB_6:
	mov rbx, rsi
	mov esi, 4
	mov r14, rdx
	mov rdx, rcx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, rbx
	mov rdx, r14
	test rax, rax
	jne .LBB_3
	xor eax, eax
	add rsp, 8
	pop rbx
	pop r14
	ret